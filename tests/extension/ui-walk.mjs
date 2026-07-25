// Walks the popup through its main pages with a logged-in wallet and reports
// any command the service worker rejected. Run with: node tests/extension/ui-walk.mjs
import { chromium } from 'playwright';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-walk-'));

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium',
  headless: true,
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

const failures = [];

try {
  let worker = context.serviceWorkers()[0];
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });
  worker.on('console', (m) => {
    if (m.type() === 'error') failures.push(`[worker] ${m.text()}`);
  });

  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  page.on('console', (m) => {
    if (m.type() === 'error') failures.push(`[popup] ${m.text()}`);
  });
  page.on('requestfailed', (request) => {
    failures.push(`[request] ${request.url()} :: ${request.failure()?.errorText}`);
  });

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

  const { data: mnemonic } = await send('generate_mnemonic', { use_24_words: true });
  const { data: imported } = await send('import_key', {
    name: 'Walk',
    key: mnemonic.mnemonic,
    save_secrets: true,
    login: true,
  });
  await send('login', { fingerprint: imported.fingerprint });

  await page.reload();
  await page.waitForSelector('#root > *', { timeout: 20000 });
  await page.waitForTimeout(3000);

  const routes = [
    '#/wallet',
    '#/transactions',
    '#/nfts',
    '#/dids',
    '#/offers',
    '#/settings',
    '#/settings?tab=wallet',
    '#/settings?tab=network',
  ];

  for (const route of routes) {
    const before = failures.length;
    await page.goto(`chrome-extension://${id}/popup.html${route}`);
    await page.waitForTimeout(2500);
    const added = failures.slice(before);
    console.log(`${route}: ${added.length ? `${added.length} error(s)` : 'clean'}`);
  }

  console.log('\n=== failures ===');
  if (failures.length === 0) {
    console.log('none');
  } else {
    for (const failure of [...new Set(failures)]) console.log(failure);
  }
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
