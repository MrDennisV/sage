// Checks that the token catalog reaches the browser database, which is where
// the interface gets the assets it offers to pick from when swapping or making
// an offer. Run with: node tests/extension/cat-catalog-walk.mjs
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
    name: 'Catalog Test',
    key: mnemonic,
    save_secrets: true,
    login: true,
  });
  await check('login', { fingerprint });
  console.log('logged in, fingerprint:', fingerprint);

  const { kind } = await check('get_network', {});
  console.log('network kind:', kind);

  const tokenCount = async () => {
    const result = JSON.parse(
      await worker.evaluate(() =>
        // Kind 0 is a token, and row 0 is the built-in XCH entry.
        globalThis.dbQuery(
          'SELECT COUNT(*) FROM assets WHERE kind = 0 AND id != 0',
          '[]',
        ),
      ),
    );
    return result.rows[0]?.[0]?.value ?? 0;
  };

  // Each sync pass walks as many catalog pages as its budget allows, so the
  // count climbs over several passes rather than arriving all at once.
  let tokens = 0;
  const deadline = Date.now() + 180000;

  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 5000));

    const current = await tokenCount();

    if (current !== tokens) {
      console.log(`tokens recorded: ${current}`);
      tokens = current;
    }

    if (tokens >= 100) break;
  }

  if (tokens < 100) {
    throw new Error(
      `only ${tokens} tokens reached the database; the catalog never landed`,
    );
  }

  // The interface reads the catalog through this command, so it has to see the
  // same tokens the database holds.
  const { cats } = await check('get_all_cats', {});
  console.log(`get_all_cats returned ${cats.length} tokens`);

  if (cats.length === 0) {
    throw new Error('get_all_cats returned nothing despite a filled catalog');
  }

  const named = cats.filter((cat) => cat.name).length;
  console.log(`${named} of them carry a name`);

  if (named === 0) {
    throw new Error('no token carries a name, so the listing was not decoded');
  }

  console.log('CAT CATALOG WALK PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
