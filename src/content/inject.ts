// Content script injected into MAIN world
// Provides window.chia and window.sage for dApp integration (CHIP-0002)

interface ChiaProvider {
  isGoby: boolean;
  isSage: boolean;
  // CHIP-002 core
  connect: (params?: any) => Promise<any>;
  disconnect: () => Promise<void>;
  chainId: () => Promise<string>;
  getPublicKeys: (params?: any) => Promise<string[]>;
  filterUnlockedCoins: (params: any) => Promise<string[]>;
  getAssetCoins: (params: any) => Promise<any[]>;
  getAssetBalance: (params: any) => Promise<any>;
  signCoinSpends: (params: any) => Promise<any>;
  signMessage: (params: any) => Promise<any>;
  sendTransaction: (params: any) => Promise<any>;
  // chia_ high-level
  getAddress: () => Promise<any>;
  getNfts: (params?: any) => Promise<any>;
  send: (params: any) => Promise<any>;
  signMessageByAddress: (params: any) => Promise<any>;
  createOffer: (params: any) => Promise<any>;
  takeOffer: (params: any) => Promise<any>;
  cancelOffer: (params: any) => Promise<any>;
  bulkMintNfts: (params: any) => Promise<any>;
  // Events
  on: (event: string, callback: (...args: any[]) => void) => void;
  off: (event: string, callback: (...args: any[]) => void) => void;
}

const eventListeners: Record<string, Set<(...args: any[]) => void>> = {};

function createProvider(): ChiaProvider {
  return {
    // Goby compatibility flag — many dApps check for this
    isGoby: true,
    isSage: true,

    // CHIP-002 core methods
    async connect(params?: any) {
      return sendDappMessage('connect', params);
    },
    async disconnect() {
      await sendDappMessage('disconnect');
    },
    async chainId() {
      return sendDappMessage('chip0002_chainId');
    },
    async getPublicKeys(params?: any) {
      return sendDappMessage('getPublicKeys', params);
    },
    async filterUnlockedCoins(params: any) {
      return sendDappMessage('chip0002_filterUnlockedCoins', params);
    },
    async getAssetCoins(params: any) {
      return sendDappMessage('chip0002_getAssetCoins', params);
    },
    async getAssetBalance(params: any) {
      return sendDappMessage('chip0002_getAssetBalance', params);
    },
    async signCoinSpends(params: any) {
      return sendDappMessage('signCoinSpends', params);
    },
    async signMessage(params: any) {
      return sendDappMessage('signMessage', params);
    },
    async sendTransaction(params: any) {
      return sendDappMessage('sendTransaction', params);
    },
    // chia_ high-level methods
    async getAddress() {
      return sendDappMessage('chia_getAddress');
    },
    async getNfts(params?: any) {
      return sendDappMessage('chia_getNfts', params);
    },
    async send(params: any) {
      return sendDappMessage('chia_send', params);
    },
    async signMessageByAddress(params: any) {
      return sendDappMessage('chia_signMessageByAddress', params);
    },
    async createOffer(params: any) {
      return sendDappMessage('chia_createOffer', params);
    },
    async takeOffer(params: any) {
      return sendDappMessage('chia_takeOffer', params);
    },
    async cancelOffer(params: any) {
      return sendDappMessage('chia_cancelOffer', params);
    },
    async bulkMintNfts(params: any) {
      return sendDappMessage('chia_bulkMintNfts', params);
    },

    on(event: string, callback: (...args: any[]) => void) {
      if (!eventListeners[event]) {
        eventListeners[event] = new Set();
      }
      eventListeners[event].add(callback);
    },

    off(event: string, callback: (...args: any[]) => void) {
      if (eventListeners[event]) {
        eventListeners[event].delete(callback);
      }
    },
  };
}

// Communication with content-bridge via postMessage
let requestId = 0;
const pendingRequests = new Map<
  number,
  { resolve: (v: any) => void; reject: (e: any) => void }
>();

function sendDappMessage(method: string, params?: any): Promise<any> {
  return new Promise((resolve, reject) => {
    const id = ++requestId;
    pendingRequests.set(id, { resolve, reject });
    window.postMessage(
      {
        type: 'SAGE_DAPP_REQUEST',
        id,
        method,
        params,
      },
      '*',
    );
  });
}

// Listen for responses from content-bridge
window.addEventListener('message', (event) => {
  if (event.source !== window) return;

  if (event.data?.type === 'SAGE_DAPP_RESPONSE') {
    const { id, result, error } = event.data;
    const pending = pendingRequests.get(id);
    if (pending) {
      pendingRequests.delete(id);
      if (error) {
        pending.reject(new Error(error));
      } else {
        pending.resolve(result);
      }
    }
  }

  // Handle events from extension
  if (event.data?.type === 'SAGE_EVENT') {
    const { eventName, data } = event.data;
    const listeners = eventListeners[eventName];
    if (listeners) {
      listeners.forEach((cb) => cb(data));
    }
  }
});

// Install the provider
const provider = createProvider();
(window as any).chia = provider;
(window as any).sage = provider;

// Announce provider availability
window.dispatchEvent(new Event('chia#initialized'));
