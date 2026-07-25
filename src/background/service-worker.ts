// The extension's backend: boots the wasm wallet core over the sql.js and
// chrome.storage bridges, routes messages from the UI, and drives sync from
// chrome.alarms. All wallet logic lives in Rust; this file only wires it up.
import init, {
  sage_handle,
  sage_init,
  sage_login,
  sage_prepare_database,
  sage_session,
  sage_sync_once,
} from '../extension/wasm/sage_wasm';

import { flushKv, initKvStore } from './kv-store';
import { databaseKey, flushDb, initSqlEngine, selectDatabase } from './sql-store';

const SYNC_ALARM = 'sage-sync';
const SYNC_PERIOD_MINUTES = 0.5;

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
]);

// Commands that change which wallet or network is active, so the matching
// database has to be selected before the next query runs.
const SESSION_COMMANDS = new Set([
  'set_network',
  'set_network_override',
  'switch_wallet',
  'import_key',
]);

function session(): Session {
  return JSON.parse(sage_session());
}

/** Points the SQL bridge at the database for the active wallet and network. */
async function useSessionDatabase(): Promise<boolean> {
  const { fingerprint, network_id: networkId } = session();

  if (fingerprint === null) return false;

  if (await selectDatabase(databaseKey(fingerprint, networkId))) {
    await sage_prepare_database();
  }

  await sage_login(fingerprint);

  return true;
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
  })();

  return booted;
}

async function syncOnce(): Promise<void> {
  await boot();

  if (session().fingerprint === null) return;

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

    const response = JSON.parse(
      await sage_handle(cmd, JSON.stringify(args ?? {})),
    );

    if (SESSION_COMMANDS.has(cmd) || cmd === 'login') {
      await useSessionDatabase();
      syncOnce().catch((error) => console.error('sync failed', error));
    }

    if (FLUSH_COMMANDS.has(cmd) || cmd === 'login') {
      await flushKv();
      await flushDb();
    }

    return response;
  })()
    .then((data) => sendResponse({ data }))
    .catch((error) => sendResponse({ error: error?.message ?? String(error) }));

  return true;
});
