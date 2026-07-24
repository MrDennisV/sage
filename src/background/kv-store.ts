// The KvStore side of the wasm bridge: config and the encrypted keychain live
// in chrome.storage.local, preloaded into a snapshot so the wasm side can
// read synchronously. Writes update the snapshot and flush asynchronously.
const KV_PREFIX = 'kv_';

const snapshot = new Map<string, string>();
let pendingWrites: Promise<unknown> = Promise.resolve();

export async function initKvStore(): Promise<void> {
  const entries = await chrome.storage.local.get(null);

  for (const [key, value] of Object.entries(entries)) {
    if (key.startsWith(KV_PREFIX) && typeof value === 'string') {
      snapshot.set(key.slice(KV_PREFIX.length), value);
    }
  }

  const globals = globalThis as Record<string, unknown>;

  globals.kvRead = (name: string): string | null => snapshot.get(name) ?? null;

  globals.kvWrite = (name: string, base64: string): void => {
    snapshot.set(name, base64);
    pendingWrites = pendingWrites.then(() =>
      chrome.storage.local.set({ [KV_PREFIX + name]: base64 }),
    );
  };
}

/** Waits until every pending chrome.storage write has completed. */
export async function flushKv(): Promise<void> {
  await pendingWrites;
}
