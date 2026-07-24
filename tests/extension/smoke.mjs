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

  const extensionId = new URL(worker.url()).host;
  console.log('service worker up:', worker.url());

  const popup = await context.newPage();
  await popup.goto(`chrome-extension://${extensionId}/popup.html`);
  await popup.waitForSelector('#root > *', { timeout: 20000 });

  const title = await popup.title();
  console.log('popup rendered, title:', title);

  const keys = await popup.evaluate(
    () =>
      new Promise((resolve) => {
        chrome.runtime.sendMessage(
          { type: 'COMMAND', cmd: 'get_keys', args: { req: {} } },
          (res) => resolve(res),
        );
      }),
  );

  console.log('get_keys response:', JSON.stringify(keys));

  if (!keys || keys.error) {
    throw new Error(`get_keys failed: ${keys?.error ?? 'no response'}`);
  }

  console.log('SMOKE TEST PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
