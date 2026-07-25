// The extension's backend: boots the wasm wallet core over the sql.js and
// chrome.storage bridges, routes messages from the UI, and drives sync from
// chrome.alarms. All wallet logic lives in Rust; this file only wires it up.
import init, {
  sage_handle,
  sage_init,
  sage_login,
  sage_mounted,
  sage_prepare_database,
  sage_session,
  sage_sync_once,
  sage_sync_pending,
  sage_wallet_network,
} from '../extension/wasm/sage_wasm';

import { broadcastEvent, installDappBridge } from './dapp-bridge';
import { flushKv, initKvStore } from './kv-store';
import {
  databaseKey,
  flushDb,
  initSqlEngine,
  selectDatabase,
} from './sql-store';

const SYNC_ALARM = 'sage-sync';
const SYNC_PERIOD_MINUTES = 0.5;
const KEEPALIVE_INTERVAL_MS = 20_000;

interface Session {
  fingerprint: number | null;
  network_id: string;
}

let booted: Promise<void> | null = null;

// Commands whose writes must be durable before we respond.
const FLUSH_COMMANDS = new Set([
  'import_key',
  'delete_key',
  'rename_key',
  'set_wallet_emoji',
  'logout',
  'set_network',
  'send_xch',
  'send_cat',
  'resync',
  'delete_database',
]);

// Commands after which the wallet has to be rebuilt over its database: either
// the active wallet or network changed, or the database was emptied and the
// records have to be synced again.
const SESSION_COMMANDS = new Set([
  'set_network',
  'set_network_override',
  'switch_wallet',
  'import_key',
  'resync',
  'delete_database',
]);

// Session changes that connected websites have to hear about, because their
// provider is holding an address or a chain id that just became wrong.
const ACCOUNT_COMMANDS = new Set([
  'switch_wallet',
  'import_key',
  'login',
  'logout',
]);

const NETWORK_COMMANDS = new Set(['set_network', 'set_network_override']);

function session(): Session {
  return JSON.parse(sage_session());
}

/** Points the SQL bridge at a wallet's database, applying the schema if new. */
async function mountDatabase(fingerprint: number, networkId: string) {
  if (await selectDatabase(databaseKey(fingerprint, networkId))) {
    await sage_prepare_database();
  }

  sage_mounted(fingerprint);
}

/** Points the SQL bridge at the database for the active wallet and network. */
async function useSessionDatabase(): Promise<boolean> {
  const { fingerprint, network_id: networkId } = session();

  if (fingerprint === null) return false;

  await mountDatabase(fingerprint, networkId);
  await sage_login(fingerprint);

  return true;
}

// Resyncing or emptying a database acts on a wallet the user names, which is
// not always the one they are signed into: both are offered from the wallet
// list. Desktop opens that wallet's file by path; here only one database is
// mounted at a time, so the named one takes its place for the command and the
// session's own is restored afterwards.
const WALLET_COMMANDS = new Set(['resync', 'delete_database']);

async function withWalletDatabase<T>(
  fingerprint: number,
  work: () => Promise<T>,
): Promise<T> {
  const { fingerprint: active } = session();

  if (active === fingerprint) return work();

  try {
    await mountDatabase(fingerprint, sage_wallet_network(fingerprint));

    return await work();
  } finally {
    await flushDb();
    await useSessionDatabase();
  }
}

function boot(): Promise<void> {
  booted ??= (async () => {
    await initSqlEngine();
    await initKvStore();
    await init({
      module_or_path: chrome.runtime.getURL('wasm/sage_wasm_bg.wasm'),
    });
    await sage_init('');
    await useSessionDatabase();

    // The worker restarts far more often than the wallet changes, so record
    // what is active up front rather than announcing it as news.
    announcedFingerprint = session().fingerprint;
  })();

  return booted;
}

// The wasm module holds one wallet over one sql.js connection, so commands and
// syncs have to take turns; overlapping them would interleave transactions.
// Chrome also stops a service worker after 30 seconds without extension API
// activity, which aborts in-flight requests, so the worker is kept alive while
// the turn runs.
let queue: Promise<unknown> = Promise.resolve();

function exclusive<T>(work: () => Promise<T>): Promise<T> {
  const run = queue.then(async () => {
    const timer = setInterval(() => {
      chrome.runtime.getPlatformInfo().catch(() => {});
    }, KEEPALIVE_INTERVAL_MS);

    try {
      return await work();
    } finally {
      clearInterval(timer);
    }
  });

  queue = run.catch(() => {});

  return run;
}

/** Boots the wallet if needed and runs `work` with sole access to it. */
function withWallet<T>(work: () => Promise<T>): Promise<T> {
  return exclusive(async () => {
    await boot();

    return work();
  });
}

/** Runs one Sage command. Only valid while holding the wallet. */
async function dispatch(cmd: string, request: unknown): Promise<unknown> {
  return JSON.parse(await sage_handle(cmd, JSON.stringify(request ?? {})));
}

async function flushAll(): Promise<void> {
  await flushKv();
  await flushDb();
}

// Opening the wallet boots its interface, which logs into whatever wallet is
// already active. That is not a change, and telling a connected site otherwise
// makes it drop the session it is in the middle of using — which is what
// approving a request would do, since answering one opens the wallet.
let announcedFingerprint: number | null = null;

function announceAccount() {
  const { fingerprint } = session();

  if (fingerprint === announcedFingerprint) return;

  announcedFingerprint = fingerprint;

  broadcastEvent('accountChanged').catch(() => {
    // Websites that missed the event will read the new wallet anyway.
  });
}

function broadcastSyncEvent(data: unknown) {
  chrome.runtime.sendMessage({ type: 'SYNC_EVENT', data }).catch(() => {
    // No listeners while the popup is closed.
  });
}

/**
 * Whether a command broadcasts, which is what leaves a pending transaction
 * behind for the interface to show. Building a transaction without submitting
 * it changes nothing yet, so it does not count.
 */
function submits(cmd: string, request: unknown): boolean {
  if (cmd === 'submit_transaction') return true;

  return (
    typeof request === 'object' &&
    request !== null &&
    (request as { auto_submit?: boolean }).auto_submit === true
  );
}

async function syncPass(): Promise<boolean> {
  const { events, pending } = await exclusive(async () => {
    await boot();

    if (session().fingerprint === null) return { events: [], pending: false };

    const events: unknown[] = JSON.parse(await sage_sync_once(true));
    await flushDb();

    return { events, pending: sage_sync_pending() };
  });

  for (const event of events) {
    chrome.runtime
      .sendMessage({ type: 'SYNC_EVENT', data: event })
      .catch(() => {
        // No listeners while the popup is closed.
      });
  }

  return pending;
}

// A pass bounds each stage so a command arriving mid-sync isn't left waiting,
// which means a wallet with a lot to catch up on needs several. Waiting for the
// timer between them would stretch a first sync over many minutes, so passes
// follow each other while there is work left. The cap is what keeps a stage
// that always reports work from taking the worker over entirely.
const MAX_CHAINED_PASSES = 20;

let syncing = false;

async function syncOnce(): Promise<void> {
  if (syncing) return;

  syncing = true;

  try {
    for (let pass = 0; pass < MAX_CHAINED_PASSES; pass++) {
      if (!(await syncPass())) return;
    }
  } finally {
    syncing = false;
  }
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(SYNC_ALARM, { periodInMinutes: SYNC_PERIOD_MINUTES });
});

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === SYNC_ALARM) {
    syncOnce().catch((error) => console.error('sync failed', error));
  }
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type !== 'COMMAND') return false;

  const { cmd, args } = message as { cmd: string; args?: { req?: unknown } };

  withWallet(async () => {
    // Tauri commands take the request wrapped in a `req` field; the wasm
    // dispatch takes the request struct itself.
    const request = args && 'req' in args ? args.req : args;

    const fingerprint = (request as { fingerprint?: number } | undefined)
      ?.fingerprint;

    const response =
      WALLET_COMMANDS.has(cmd) && typeof fingerprint === 'number'
        ? await withWalletDatabase(fingerprint, () => dispatch(cmd, request))
        : await dispatch(cmd, request);

    if (SESSION_COMMANDS.has(cmd) || cmd === 'login') {
      await useSessionDatabase();
    }

    if (FLUSH_COMMANDS.has(cmd) || cmd === 'login') {
      await flushAll();
    }

    if (submits(cmd, request)) await flushDb();

    return response;
  })
    .then((response) => {
      // Submitting records the transaction as pending, and the wallet shows it
      // straight away. Nothing drives the interface here the way the desktop
      // sync manager does, so the worker says so itself rather than leaving it
      // to look idle until the next pass.
      const request = args && 'req' in args ? args.req : args;
      const broadcast = submits(cmd, request);

      if (broadcast) {
        broadcastSyncEvent({ type: 'coin_state' });
      }

      if (SESSION_COMMANDS.has(cmd) || cmd === 'login' || broadcast) {
        syncOnce().catch((error) => console.error('sync failed', error));
      }

      if (ACCOUNT_COMMANDS.has(cmd)) {
        announceAccount();
      }

      if (NETWORK_COMMANDS.has(cmd)) {
        broadcastEvent('chainChanged').catch(() => {
          // Websites that missed the event will read the new chain anyway.
        });
      }

      return response;
    })
    .then((data) => sendResponse({ data }))
    .catch((error) => {
      const reason = error?.message ?? String(error);
      console.error(`command ${message.cmd} failed:`, reason);
      sendResponse({ error: reason });
    });

  return true;
});

installDappBridge({
  withWallet,
  dispatch,
  isLoggedIn: () => session().fingerprint !== null,
  refreshSession: useSessionDatabase,
  flush: flushAll,
  sync: () => syncOnce().catch((error) => console.error('sync failed', error)),
});
