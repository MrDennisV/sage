// Smoke test: loads the unpacked extension, waits for the service worker to
// boot the wasm core, and checks that the popup renders and the worker
// answers a command. Run with: node tests/extension/smoke.mjs
import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const dist = resolve(root, 'dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-ext-'));

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium',
  headless: true,
  args: [
    `--disable-extensions-except=${dist}`,
    `--load-extension=${dist}`,
  ],
});

try {
  let [worker] = context.serviceWorkers();
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });

  const workerErrors = [];
  worker.on('console', (message) => {
    if (message.type() === 'error') workerErrors.push(message.text());
  });

  const extensionId = new URL(worker.url()).host;
  console.log('service worker up:', worker.url());

  const popup = await context.newPage();
  await popup.goto(`chrome-extension://${extensionId}/popup.html`);
  await popup.waitForSelector('#root > *', { timeout: 20000 });

  const title = await popup.title();
  console.log('popup rendered, title:', title);

  // Drives the wallet through the service worker the same way the UI does.
  const send = (cmd, req = {}) =>
    popup.evaluate(
      ([cmd, req]) =>
        new Promise((resolve) => {
          chrome.runtime.sendMessage({ type: 'COMMAND', cmd, args: { req } }, resolve);
        }),
      [cmd, req],
    );

  const check = async (cmd, req) => {
    const response = await send(cmd, req);
    if (!response || response.error) {
      throw new Error(`${cmd} failed: ${response?.error ?? 'no response'}`);
    }
    return response.data;
  };

  const { keys } = await check('get_keys');
  console.log(`get_keys: ${keys.length} keys`);

  const { mnemonic } = await check('generate_mnemonic', { use_24_words: true });
  console.log('generated a mnemonic');

  const { fingerprint } = await check('import_key', {
    name: 'Smoke Test',
    key: mnemonic,
    save_secrets: true,
    login: true,
  });
  console.log('imported key, fingerprint:', fingerprint);

  await check('login', { fingerprint });
  console.log('logged in');

  const key = await check('get_key', {});
  if (key.key?.fingerprint !== fingerprint) {
    throw new Error(`active key is ${JSON.stringify(key.key)}, expected ${fingerprint}`);
  }
  console.log('active key confirmed');

  const sync = await check('get_sync_status', {});
  console.log('sync status:', JSON.stringify(sync));

  const { networks } = await check('get_networks', {});
  console.log('networks available:', networks.map((n) => n.name).join(', '));

  const { derivations } = await check('get_derivations', { offset: 0, limit: 3 });
  console.log(`derivations: ${derivations.length}, first: ${derivations[0]?.address}`);

  if (derivations.length === 0) {
    throw new Error('no derivations were created for the imported key');
  }

  // The sync runs in the background after login; give it time to reach the
  // Coinset API so a failing request shows up as a worker error.
  await new Promise((r) => setTimeout(r, 15000));

  if (workerErrors.length > 0) {
    throw new Error(`service worker errors:\n${workerErrors.join('\n')}`);
  }

  // The peak only lands in the database if the sync reached the Coinset API
  // and wrote through the wasm executor, so it proves the whole path.
  const blocks = JSON.parse(
    await worker.evaluate(() =>
      globalThis.dbQuery('SELECT MAX(height) AS peak FROM blocks', '[]'),
    ),
  );

  const peak = blocks.rows[0]?.[0]?.value ?? 0;
  console.log('network peak recorded in the database:', peak);

  if (peak <= 0) {
    throw new Error('the sync did not record a network peak');
  }

  console.log('SMOKE TEST PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
