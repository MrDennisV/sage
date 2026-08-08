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

  // theme-o-rama's applyThemeIsolated writes `url(undefined)` for a theme with
  // no background image, so every theme card on the settings page asks for a
  // file that was never there. Desktop does the same; the request just fails
  // visibly here because the extension origin serves nothing back.
  const missingThemeImage = (url) => url?.endsWith('/undefined') ?? false;

  // The blur that covers sensitive content registers a CSS paint worklet from
  // a blob, which an extension's content security policy does not allow. The
  // library falls back to a plain fill, so the content stays covered.
  const blockedPaintWorklet = (text) =>
    text.includes("Loading the script 'blob:") &&
    text.includes('Content Security Policy');

  page.on('console', (m) => {
    if (m.type() !== 'error') return;
    if (missingThemeImage(m.location()?.url)) return;
    if (blockedPaintWorklet(m.text())) return;

    failures.push(`[popup] ${m.text()}`);
  });
  page.on('requestfailed', (request) => {
    if (missingThemeImage(request.url())) return;

    failures.push(
      `[request] ${request.url()} :: ${request.failure()?.errorText}`,
    );
  });

  await page.goto(`chrome-extension://${id}/popup.html`);
  await page.waitForSelector('#root > *', { timeout: 20000 });

  const send = (cmd, req = {}) =>
    page.evaluate(
      ([c, r]) =>
        new Promise((res) =>
          chrome.runtime.sendMessage(
            { type: 'COMMAND', cmd: c, args: { req: r } },
            res,
          ),
        ),
      [cmd, req],
    );

  const { data: mnemonic } = await send('generate_mnemonic', {
    use_24_words: true,
  });
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
    '#/wallet/send/xch',
    '#/wallet/receive/xch',
    '#/transactions',
    '#/nfts',
    '#/dids',
    '#/offers',
    '#/offers/make',
    '#/settings',
    '#/settings?tab=wallet',
    '#/settings?tab=network',
  ];

  for (const route of routes) {
    const before = failures.length;
    await page.goto(`chrome-extension://${id}/popup.html${route}`);
    await page.waitForTimeout(2500);

    // Typing an address exercises validation, which the send page runs on
    // every keystroke.
    const address = page.locator('input').first();
    if (route.includes('/send/') && (await address.count())) {
      await address.fill(
        'xch1qkludcemyh7x0wnxy65ukj5uh3eqgve4zgjun7hmt9n6mgprw6kqqxx6yz',
      );
      await page.waitForTimeout(1500);
    }

    const added = failures.slice(before);
    console.log(
      `${route}: ${added.length ? `${added.length} error(s)` : 'clean'}`,
    );
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
