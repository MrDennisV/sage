// Serves `window.chia` for web pages. Requests arrive from the content bridge,
// are translated out of Goby's vocabulary, and then take one of two paths:
// read-only commands run here against the wasm wallet, while anything that
// spends, signs or grants access is parked until the user answers Sage's own
// confirmation dialog in the popup.
import { commands } from '@/bindings';
import {
  walletConnectCommands,
  type WalletConnectCommand,
} from '@/walletconnect/commands';
import { toDappRequest, type NativeMethod } from '@/walletconnect/goby';
import { handleCommand } from '@/walletconnect/handler';
import {
  setCommandDispatcher,
  type CommandDispatcher,
} from '../extension/command-dispatcher';
import {
  grantOrigin,
  isOriginPermitted,
  revokeOrigin,
} from './permission-manager';

/** The parts of the service worker the bridge needs to reach the wallet. */
export interface WalletRuntime {
  /** Runs work with sole access to the booted wasm wallet. */
  withWallet: <T>(work: () => Promise<T>) => Promise<T>;
  /** Runs a single Sage command. Only valid inside `withWallet`. */
  dispatch: CommandDispatcher;
  /** Whether a wallet is logged in. Only valid inside `withWallet`. */
  isLoggedIn: () => boolean;
  /** Re-selects the per-wallet, per-network database. Only valid inside `withWallet`. */
  refreshSession: () => Promise<unknown>;
  /** Makes pending writes durable. Only valid inside `withWallet`. */
  flush: () => Promise<void>;
  /** Kicks off a sync pass after the session changed. */
  sync: () => void;
}

const UI_PORT = 'sage-dapp-ui';

/** A dApp should not be left hanging if the user walks away from the popup. */
const APPROVAL_TIMEOUT_MS = 5 * 60 * 1000;

/** Closing the popup rejects, but ignore the flicker of a popup being replaced. */
const UI_CLOSE_GRACE_MS = 500;

/** How long to wait for the action popup before falling back to a window. */
const UI_OPEN_TIMEOUT_MS = 1500;

/** How long a settled popup waits before closing, in case more work arrives. */
const UI_DISMISS_DELAY_MS = 1000;

/**
 * Chrome stops an idle service worker after 30 seconds, which would drop a
 * request the user is still reading. Touching an extension API keeps it up
 * while anything is waiting on them.
 */
const KEEPALIVE_INTERVAL_MS = 20_000;

interface PendingRequest {
  id: string;
  method: WalletConnectCommand;
  params: unknown;
  origin: string;
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

const pending = new Map<string, PendingRequest>();
const uiPorts = new Set<chrome.runtime.Port>();
const uiWaiters = new Set<() => void>();
const closeWaiters = new Set<() => void>();

let nextRequestId = 0;
let approvalWindowId: number | null = null;

// Whether the bridge is the reason a wallet window is on screen. A popup the
// user opened themselves is theirs to close.
let openedUi = false;

// A close the bridge asked for, still in flight. The disconnect it produces is
// not the user walking away, so it must not take pending requests down with it.
let dismissTimer: ReturnType<typeof setTimeout> | null = null;
let dismissing = false;
let keepalive: ReturnType<typeof setInterval> | null = null;
let runtime: WalletRuntime;

function updateKeepalive() {
  if (pending.size > 0 && keepalive === null) {
    keepalive = setInterval(() => {
      chrome.runtime.getPlatformInfo().catch(() => {
        // Only the call matters; the answer is thrown away.
      });
    }, KEEPALIVE_INTERVAL_MS);
  } else if (pending.size === 0 && keepalive !== null) {
    clearInterval(keepalive);
    keepalive = null;
  }
}

// ─── Wallet access ──────────────────────────────────────────────────

/**
 * Runs handlers that call `commands.*` inside the service worker. The bindings
 * adapter would normally message the service worker, which from here would
 * deadlock, so a direct dispatcher is installed for the duration of the turn.
 */
function inWallet<T>(work: () => Promise<T>): Promise<T> {
  return runtime.withWallet(async () => {
    if (!runtime.isLoggedIn()) {
      throw new Error('No wallet is logged in');
    }

    setCommandDispatcher(runtime.dispatch);

    try {
      return await work();
    } finally {
      setCommandDispatcher(null);
    }
  });
}

/** Rejects if there is no wallet to serve a request with. */
function requireWallet(): Promise<void> {
  return runtime.withWallet(async () => {
    if (!runtime.isLoggedIn()) {
      throw new Error('No wallet is logged in');
    }
  });
}

// ─── Approval round trip ────────────────────────────────────────────

function describePending(request: PendingRequest) {
  return {
    id: request.id,
    method: request.method,
    params: request.params,
    origin: request.origin,
  };
}

function broadcastPending() {
  const [request] = pending.values();
  const message = {
    type: 'DAPP_PENDING',
    request: request ? describePending(request) : null,
  };

  for (const port of uiPorts) {
    try {
      port.postMessage(message);
    } catch {
      // The popup closed between the check and the send.
    }
  }
}

function waitForUi(): Promise<boolean> {
  if (uiPorts.size > 0) return Promise.resolve(true);

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      uiWaiters.delete(notify);
      resolve(false);
    }, UI_OPEN_TIMEOUT_MS);

    const notify = () => {
      clearTimeout(timer);
      uiWaiters.delete(notify);
      resolve(true);
    };

    uiWaiters.add(notify);
  });
}

/** Waits for a close already under way, so the next popup starts from nothing. */
function waitForUiToClose(): Promise<void> {
  if (!dismissing || uiPorts.size === 0) return Promise.resolve();

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      closeWaiters.delete(notify);
      resolve();
    }, UI_CLOSE_GRACE_MS);

    const notify = () => {
      clearTimeout(timer);
      closeWaiters.delete(notify);
      resolve();
    };

    closeWaiters.add(notify);
  });
}

/**
 * Brings up Sage's own popup. `chrome.action.openPopup` reuses the toolbar
 * popup where the browser allows it; otherwise the same page is opened as a
 * window. Never a bespoke approval page.
 */
async function openApprovalUi(): Promise<void> {
  // A popup we already told to close is not one to hand a new request to, so
  // wait for it to go before opening a fresh one.
  if (dismissing) await waitForUiToClose();

  if (uiPorts.size > 0) return;

  openedUi = true;

  try {
    await chrome.action.openPopup();

    if (await waitForUi()) return;
  } catch {
    // Older browsers, no focused window, or the popup was suppressed.
  }

  if (approvalWindowId !== null) return;

  const created = await chrome.windows.create({
    url: chrome.runtime.getURL('popup.html'),
    type: 'popup',
    width: 420,
    height: 640,
    focused: true,
  });

  approvalWindowId = created?.id ?? null;
}

/**
 * Closes the popup the bridge opened, once nothing is left to answer.
 *
 * A site commonly follows one request straight with another — sign, then send
 * — so the close waits a moment first. Without that, the second request finds a
 * popup that is already closing: it looks open, so no new one is opened, and
 * then the disconnect arrives and takes the request down with it.
 */
function dismissOpenedUi() {
  if (!openedUi || pending.size > 0 || dismissTimer !== null) return;

  dismissTimer = setTimeout(() => {
    dismissTimer = null;

    // Something arrived while we waited, and it needs this popup.
    if (pending.size > 0) return;

    openedUi = false;
    dismissing = true;

    for (const port of uiPorts) {
      try {
        port.postMessage({ type: 'DAPP_CLOSE' });
      } catch {
        // Already gone.
      }
    }
  }, UI_DISMISS_DELAY_MS);
}

/** Calls off a close that has not happened yet, because there is work again. */
function keepUiOpen() {
  if (dismissTimer === null) return;

  clearTimeout(dismissTimer);
  dismissTimer = null;
}

function closeApprovalWindow() {
  if (approvalWindowId === null || pending.size > 0) return;

  const windowId = approvalWindowId;
  approvalWindowId = null;
  chrome.windows.remove(windowId).catch(() => {
    // The user already closed it.
  });
}

function settle(request: PendingRequest, error?: Error, result?: unknown) {
  pending.delete(request.id);
  clearTimeout(request.timeout);
  updateKeepalive();

  if (error) {
    request.reject(error);
  } else {
    request.resolve(result);
  }

  broadcastPending();
  closeApprovalWindow();
  dismissOpenedUi();
}

function rejectAllPending(reason: string) {
  for (const request of [...pending.values()]) {
    settle(request, new Error(reason));
  }
}

async function requestApproval(
  method: WalletConnectCommand,
  params: unknown,
  origin: string,
): Promise<unknown> {
  // Fail before opening any UI if the wallet cannot serve the request anyway.
  await requireWallet();

  const id = `dapp-${++nextRequestId}`;

  const approval = new Promise<unknown>((resolve, reject) => {
    const timeout = setTimeout(() => {
      const request = pending.get(id);
      if (request) settle(request, new Error('The request timed out'));
    }, APPROVAL_TIMEOUT_MS);

    pending.set(id, { id, method, params, origin, resolve, reject, timeout });
  });

  keepUiOpen();
  updateKeepalive();
  broadcastPending();

  try {
    await openApprovalUi();
  } catch (error) {
    const request = pending.get(id);
    if (request) {
      settle(
        request,
        error instanceof Error ? error : new Error(String(error)),
      );
    }
  }

  broadcastPending();

  return approval;
}

/** The popup finished a request: `handleCommand` already ran over there. */
async function onApprovalResult(message: {
  id?: string;
  result?: unknown;
  error?: string;
}) {
  const request = message.id ? pending.get(message.id) : undefined;

  if (!request) return;

  if (message.error) {
    settle(request, new Error(message.error));
    return;
  }

  // Connecting is the one approval that also changes what the origin may do.
  if (request.method === 'chip0002_connect') {
    await grantOrigin(request.origin);
  }

  settle(request, undefined, message.result);
}

// ─── Goby methods with no WalletConnect command behind them ─────────

async function switchChain(params: unknown): Promise<null> {
  const chainId = (params as { chainId?: string } | undefined)?.chainId;

  if (!chainId) {
    throw new Error('walletSwitchChain requires a chainId');
  }

  await inWallet(async () => {
    const { networks } = await commands.getNetworks({});
    const network = networks.find(
      (candidate) => (candidate.network_id ?? candidate.name) === chainId,
    );

    if (!network) {
      throw new Error(`Unknown chain: ${chainId}`);
    }

    await commands.setNetwork({ name: network.name });
    // Each network has its own database, exactly as for a `set_network`
    // command from the popup.
    await runtime.refreshSession();
    await runtime.flush();
  });

  runtime.sync();
  await broadcastEvent('chainChanged');

  return null;
}

async function watchAsset(params: unknown): Promise<boolean> {
  const { type, options } = (params ?? {}) as {
    type?: string;
    options?: { assetId?: string; symbol?: string; logo?: string };
  };

  if (type && type.toUpperCase() !== 'CAT') {
    throw new Error(`walletWatchAsset only supports CAT assets, not ${type}`);
  }

  const assetId = options?.assetId;

  if (!assetId) {
    throw new Error('walletWatchAsset requires options.assetId');
  }

  return inWallet(async () => {
    const { token } = await commands.getToken({ asset_id: assetId });

    // Sage only tracks assets the wallet has actually seen, so there is no
    // faithful way to start watching an unknown one.
    if (!token) {
      throw new Error(`Asset ${assetId} is not known to this wallet`);
    }

    await commands.updateCat({
      record: {
        ...token,
        ticker: options.symbol ?? token.ticker,
        icon_url: options.logo ?? token.icon_url,
        visible: true,
      },
    });
    await runtime.flush();

    return true;
  });
}

async function handleNativeMethod(
  method: NativeMethod,
  params: unknown,
  origin: string,
): Promise<unknown> {
  switch (method) {
    case 'disconnect':
      await revokeOrigin(origin);
      return null;
    case 'walletSwitchChain':
      return switchChain(params);
    case 'walletWatchAsset':
      return watchAsset(params);
  }
}

// ─── Request routing ────────────────────────────────────────────────

async function connect(params: unknown, origin: string): Promise<boolean> {
  if (await isOriginPermitted(origin)) return true;

  // An eager connect is a dApp asking "am I already trusted?" on page load,
  // and must never raise a prompt.
  if ((params as { eager?: boolean } | undefined)?.eager) return false;

  return (await requestApproval('chip0002_connect', params, origin)) as boolean;
}

async function route(
  method: WalletConnectCommand | NativeMethod,
  params: unknown,
  origin: string,
): Promise<unknown> {
  if (method === 'chip0002_connect') {
    return connect(params, origin);
  }

  if (method === 'disconnect') {
    return handleNativeMethod(method, params, origin);
  }

  if (!(await isOriginPermitted(origin))) {
    throw new Error(`${origin} is not connected. Call connect() first.`);
  }

  if (method === 'walletSwitchChain' || method === 'walletWatchAsset') {
    return handleNativeMethod(method, params, origin);
  }

  const command = walletConnectCommands[method];

  if (!command) {
    throw new Error(`Unsupported method: ${method}`);
  }

  try {
    command.paramsType.parse(params);
  } catch (error) {
    throw new Error(
      error instanceof Error
        ? error.message
        : `Invalid parameters for ${method}`,
    );
  }

  if (command.confirm) {
    return requestApproval(method, params, origin);
  }

  // Read-only: the same handler the popup would run, minus the dialog.
  return inWallet(() =>
    handleCommand(method, params, { promptIfEnabled: async () => true }),
  );
}

async function handleDappRequest(
  message: { method?: string; params?: unknown },
  sender: chrome.runtime.MessageSender,
): Promise<unknown> {
  const origin =
    sender.origin ?? (sender.url ? new URL(sender.url).origin : '');

  if (!origin) {
    throw new Error('Could not determine the requesting origin');
  }

  const request = toDappRequest(message.method ?? '', message.params);

  // Goby's methods are called with no arguments where Sage's schemas expect an
  // empty request object.
  return route(request.method, request.params ?? {}, origin);
}

// ─── Events ─────────────────────────────────────────────────────────

/** Tells every page's provider that the wallet or the network moved. */
export async function broadcastEvent(eventName: string): Promise<void> {
  const tabs = await chrome.tabs.query({});

  await Promise.all(
    tabs.map((tab) =>
      tab.id === undefined
        ? Promise.resolve()
        : chrome.tabs
            .sendMessage(tab.id, { type: 'SAGE_EVENT', eventName })
            .catch(() => {
              // No content script on that tab.
            }),
    ),
  );
}

// ─── Wiring ─────────────────────────────────────────────────────────

export function installDappBridge(walletRuntime: WalletRuntime): void {
  runtime = walletRuntime;

  chrome.runtime.onConnect.addListener((port) => {
    if (port.name !== UI_PORT) return;

    uiPorts.add(port);

    for (const notify of [...uiWaiters]) notify();

    const [request] = pending.values();
    port.postMessage({
      type: 'DAPP_PENDING',
      request: request ? describePending(request) : null,
    });

    port.onDisconnect.addListener(() => {
      uiPorts.delete(port);

      // We asked for this one. Anything pending arrived after the close was
      // already on its way, so it gets a popup of its own instead of an error.
      if (dismissing && uiPorts.size === 0) {
        dismissing = false;

        for (const notify of [...closeWaiters]) notify();

        if (pending.size > 0) {
          openApprovalUi().catch(() => {
            // The reopen failed; the timeout is what gives up on the request.
          });
        }

        return;
      }

      setTimeout(() => {
        if (uiPorts.size === 0) {
          rejectAllPending(
            'The wallet was closed before the request was approved',
          );
        }
      }, UI_CLOSE_GRACE_MS);
    });
  });

  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message?.type === 'DAPP_REQUEST') {
      handleDappRequest(message, sender)
        .then((data) => sendResponse({ data }))
        .catch((error) => {
          const reason = error?.message ?? String(error);
          console.warn(`dApp request ${message.method} failed:`, reason);
          sendResponse({ error: reason });
        });

      return true;
    }

    if (message?.type === 'DAPP_RESULT') {
      onApprovalResult(message).catch((error) =>
        console.error('failed to settle dApp request', error),
      );
      sendResponse({ data: null });

      return false;
    }

    return false;
  });

  chrome.windows.onRemoved.addListener((windowId) => {
    if (windowId !== approvalWindowId) return;

    approvalWindowId = null;
    rejectAllPending('The wallet was closed before the request was approved');
  });
}
