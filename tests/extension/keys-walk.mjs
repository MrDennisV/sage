// Exercises the key lifecycle the way the interface does: create, list,
// delete, and confirm that a failure comes back with a readable message.
import { chromium } from 'playwright';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-keys-'));

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium',
  headless: true,
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

try {
  let worker = context.serviceWorkers()[0];
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });

  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/popup.html`);
  await page.waitForSelector('#root > *', { timeout: 20000 });

  const send = (cmd, req = {}) =>
    page.evaluate(
      ([c, r]) =>
        new Promise((res) =>
          chrome.runtime.sendMessage({ type: 'COMMAND', cmd: c, args: { req: r } }, res),
        ),
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
    name: 'Lifecycle',
    key: mnemonic,
    save_secrets: true,
    login: true,
  });
  await check('login', { fingerprint });
  console.log('created wallet', fingerprint);

  // Importing the same key again must fail with a message the interface can
  // show, not an empty error.
  const duplicate = await send('import_key', {
    name: 'Lifecycle',
    key: mnemonic,
    save_secrets: true,
    login: false,
  });

  if (!duplicate?.error) {
    throw new Error('importing a duplicate key unexpectedly succeeded');
  }

  let parsed;
  try {
    parsed = JSON.parse(duplicate.error);
  } catch {
    throw new Error(`duplicate error is not structured: ${duplicate.error}`);
  }

  if (!parsed.kind || !parsed.reason) {
    throw new Error(`duplicate error lacks kind/reason: ${duplicate.error}`);
  }
  console.log(`duplicate rejected as ${parsed.kind}: ${parsed.reason}`);

  await check('delete_key', { fingerprint });
  const { keys } = await check('get_keys');

  if (keys.some((key) => key.fingerprint === fingerprint)) {
    throw new Error('the deleted key is still listed');
  }
  console.log(`deleted wallet, ${keys.length} key(s) left`);

  console.log('KEYS TEST PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
