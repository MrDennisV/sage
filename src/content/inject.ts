// Runs in the page's MAIN world and installs `window.chia`, the provider Chia
// dApps talk to. The API surface is Goby's (docs.goby.app) because that is what
// dApps are written against: `request({ method, params })` is the entry point,
// method names are bare, and `accountChanged`/`chainChanged` tell a dApp to
// reload. Every call is relayed to the content bridge, which forwards it to the
// service worker, where `walletconnect/goby.ts` translates the Goby vocabulary
// into Sage's commands. This file deliberately has no imports: it is injected
// into every page and a shared chunk would not be loadable from there.

interface RequestArguments {
  method: string;
  params?: unknown;
}

interface ChiaProvider {
  isSage: boolean;
  // Canonical entry point
  request: (args: RequestArguments) => Promise<unknown>;
  // CHIP-0002 core
  connect: (params?: unknown) => Promise<boolean>;
  disconnect: () => Promise<void>;
  chainId: () => Promise<string>;
  getPublicKeys: (params?: unknown) => Promise<string[]>;
  filterUnlockedCoins: (params: unknown) => Promise<string[]>;
  getAssetCoins: (params: unknown) => Promise<unknown[]>;
  getAssetBalance: (params: unknown) => Promise<unknown>;
  signCoinSpends: (params: unknown) => Promise<unknown>;
  signMessage: (params: unknown) => Promise<unknown>;
  sendTransaction: (params: unknown) => Promise<unknown>;
  // Goby wallet methods
  transfer: (params: unknown) => Promise<unknown>;
  walletWatchAsset: (params: unknown) => Promise<boolean>;
  walletSwitchChain: (params: unknown) => Promise<null>;
  // High level Sage methods
  getAddress: () => Promise<unknown>;
  getNfts: (params?: unknown) => Promise<unknown>;
  send: (params: unknown) => Promise<unknown>;
  signMessageByAddress: (params: unknown) => Promise<unknown>;
  createOffer: (params: unknown) => Promise<unknown>;
  takeOffer: (params: unknown) => Promise<unknown>;
  cancelOffer: (params: unknown) => Promise<unknown>;
  bulkMintNfts: (params: unknown) => Promise<unknown>;
  // Events
  on: (event: string, callback: (...args: unknown[]) => void) => void;
  off: (event: string, callback: (...args: unknown[]) => void) => void;
}

const eventListeners: Record<string, Set<(...args: unknown[]) => void>> = {};

let requestId = 0;

const pendingRequests = new Map<
  number,
  { resolve: (value: unknown) => void; reject: (error: Error) => void }
>();

function sendDappMessage(method: string, params?: unknown): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const id = ++requestId;
    pendingRequests.set(id, { resolve, reject });

    window.postMessage({ type: 'SAGE_DAPP_REQUEST', id, method, params }, '*');
  });
}

function createProvider(): ChiaProvider {
  const request = ({ method, params }: RequestArguments) =>
    sendDappMessage(method, params);

  return {
    isSage: true,

    request,

    // CHIP-0002 core methods
    async connect(params?: unknown) {
      return (await request({ method: 'connect', params })) as boolean;
    },
    async disconnect() {
      await request({ method: 'disconnect' });
    },
    async chainId() {
      return (await request({ method: 'chainId' })) as string;
    },
    async getPublicKeys(params?: unknown) {
      return (await request({ method: 'getPublicKeys', params })) as string[];
    },
    async filterUnlockedCoins(params: unknown) {
      return (await request({
        method: 'filterUnlockedCoins',
        params,
      })) as string[];
    },
    async getAssetCoins(params: unknown) {
      return (await request({ method: 'getAssetCoins', params })) as unknown[];
    },
    async getAssetBalance(params: unknown) {
      return request({ method: 'getAssetBalance', params });
    },
    async signCoinSpends(params: unknown) {
      return request({ method: 'signCoinSpends', params });
    },
    async signMessage(params: unknown) {
      return request({ method: 'signMessage', params });
    },
    async sendTransaction(params: unknown) {
      return request({ method: 'sendTransaction', params });
    },

    // Goby wallet methods
    async transfer(params: unknown) {
      return request({ method: 'transfer', params });
    },
    async walletWatchAsset(params: unknown) {
      return (await request({
        method: 'walletWatchAsset',
        params,
      })) as boolean;
    },
    async walletSwitchChain(params: unknown) {
      return (await request({ method: 'walletSwitchChain', params })) as null;
    },

    // High level Sage methods
    async getAddress() {
      return request({ method: 'getAddress' });
    },
    async getNfts(params?: unknown) {
      return request({ method: 'getNfts', params });
    },
    async send(params: unknown) {
      return request({ method: 'send', params });
    },
    async signMessageByAddress(params: unknown) {
      return request({ method: 'signMessageByAddress', params });
    },
    async createOffer(params: unknown) {
      return request({ method: 'createOffer', params });
    },
    async takeOffer(params: unknown) {
      return request({ method: 'takeOffer', params });
    },
    async cancelOffer(params: unknown) {
      return request({ method: 'cancelOffer', params });
    },
    async bulkMintNfts(params: unknown) {
      return request({ method: 'bulkMintNfts', params });
    },

    on(event: string, callback: (...args: unknown[]) => void) {
      eventListeners[event] ??= new Set();
      eventListeners[event].add(callback);
    },

    off(event: string, callback: (...args: unknown[]) => void) {
      eventListeners[event]?.delete(callback);
    },
  };
}

window.addEventListener('message', (event) => {
  if (event.source !== window) return;

  if (event.data?.type === 'SAGE_DAPP_RESPONSE') {
    const { id, result, error } = event.data;
    const pending = pendingRequests.get(id);

    if (!pending) return;

    pendingRequests.delete(id);

    if (error) {
      pending.reject(new Error(error));
    } else {
      pending.resolve(result);
    }
  }

  // Wallet or network changes the dApp needs to know about.
  if (event.data?.type === 'SAGE_EVENT') {
    const { eventName, data } = event.data;
    eventListeners[eventName]?.forEach((callback) => callback(data));
  }
});

const provider = createProvider();
(window as unknown as { chia: ChiaProvider }).chia = provider;
(window as unknown as { sage: ChiaProvider }).sage = provider;

// Announce provider availability
window.dispatchEvent(new Event('chia#initialized'));
