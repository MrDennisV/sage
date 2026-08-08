// A token, NFT or DID coin is locked to a puzzle the wallet does not own, so
// the only thing tying it to the wallet is the hint its parent spend leaves
// behind. The node protocol answers puzzle hashes and hints from one request;
// the Coinset API splits them, so the sync has to ask for both or those coins
// never arrive. Run with: node tests/extension/hinted-coins-walk.mjs
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
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

const requested = new Set();

context.on('request', (request) => {
  const path = new URL(request.url()).pathname.replace(/^\//, '');
  if (path.startsWith('get_coin_records_by')) requested.add(path);
});

try {
  let [worker] = context.serviceWorkers();
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });

  const extensionId = new URL(worker.url()).host;

  const popup = await context.newPage();
  await popup.goto(`chrome-extension://${extensionId}/popup.html`);
  await popup.waitForSelector('#root > *', { timeout: 20000 });

  const send = (cmd, req = {}) =>
    popup.evaluate(
      ([cmd, req]) =>
        new Promise((resolve) => {
          chrome.runtime.sendMessage(
            { type: 'COMMAND', cmd, args: { req } },
            resolve,
          );
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

  const { mnemonic } = await check('generate_mnemonic', { use_24_words: true });
  const { fingerprint } = await check('import_key', {
    name: 'Hinted Coins Test',
    key: mnemonic,
    save_secrets: true,
    login: true,
  });
  await check('login', { fingerprint });
  console.log('logged in, fingerprint:', fingerprint);

  // Give the sync a couple of passes to reach the Coinset API.
  const deadline = Date.now() + 60000;

  while (Date.now() < deadline && !requested.has('get_coin_records_by_hints')) {
    await new Promise((r) => setTimeout(r, 2000));
  }

  console.log('coin lookups issued:', [...requested].join(', ') || 'none');

  for (const endpoint of [
    'get_coin_records_by_puzzle_hashes',
    'get_coin_records_by_hints',
  ]) {
    if (!requested.has(endpoint)) {
      throw new Error(`the sync never called ${endpoint}`);
    }
  }

  // A wallet that asks for hinted coins and never looks them up would sync
  // XCH and silently miss every token, NFT and DID it holds.
  console.log('HINTED COINS WALK PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
