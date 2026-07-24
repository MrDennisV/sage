// The extension's backend: boots the wasm wallet core over the sql.js and
// chrome.storage bridges, routes messages from the UI, and drives sync from
// chrome.alarms. All wallet logic lives in Rust; this file only wires it up.
import init, {
  sage_handle,
  sage_init,
  sage_login,
  sage_sync_once,
} from '../extension/wasm/sage_wasm';

import { flushKv, initKvStore } from './kv-store';
import { flushDb, initDatabase } from './sql-store';

const NETWORK_ID = 'testnet11';
const SYNC_ALARM = 'sage-sync';
const SYNC_PERIOD_MINUTES = 0.5;

let booted: Promise<void> | null = null;

// Commands whose writes must be durable before we respond.
const FLUSH_COMMANDS = new Set([
  'login',
  'import_key',
  'delete_key',
  'generate_mnemonic',
  'send_xch',
  'send_cat',
]);

function boot(): Promise<void> {
  booted ??= (async () => {
    await initDatabase();
    await initKvStore();
    await init({ module_or_path: chrome.runtime.getURL('wasm/sage_wasm_bg.wasm') });
    await sage_init(NETWORK_ID);
  })();

  return booted;
}

async function syncOnce(): Promise<void> {
  await boot();

  const events: unknown[] = JSON.parse(await sage_sync_once(true));

  await flushDb();

  for (const event of events) {
    chrome.runtime.sendMessage({ type: 'SYNC_EVENT', data: event }).catch(() => {
      // No listeners while the popup is closed.
    });
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

  (async () => {
    await boot();

    const { cmd, args } = message as { cmd: string; args: unknown };

    if (cmd === 'login') {
      await sage_login(Number((args as { fingerprint: number }).fingerprint));
      syncOnce().catch((error) => console.error('sync failed', error));
      return {};
    }

    const response = JSON.parse(await sage_handle(cmd, JSON.stringify(args ?? {})));

    if (FLUSH_COMMANDS.has(cmd)) {
      await flushKv();
      await flushDb();
    }

    return response;
  })()
    .then((data) => sendResponse({ data }))
    .catch((error) => sendResponse({ error: error?.message ?? String(error) }));

  return true;
});
