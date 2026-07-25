// Fails if any Lingui message id leaks into the UI instead of its text.
import { chromium } from 'playwright';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-i18n-'));
const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium', headless: true,
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

const uncompiled = [];

try {
  let worker = context.serviceWorkers()[0];
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });
  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  page.on('console', (m) => {
    if (m.text().includes('Uncompiled message detected')) {
      uncompiled.push(m.text().split('\n').slice(0, 3).join(' ').trim());
    }
  });

  for (const route of ['#/wallet', '#/settings', '#/settings?tab=network', '#/settings?tab=wallet', '#/transactions']) {
    await page.goto(`chrome-extension://${id}/popup.html${route}`);
    await page.waitForSelector('#root > *', { timeout: 20000 });
    await page.waitForTimeout(1500);
  }

  if (uncompiled.length) {
    console.log('UNCOMPILED MESSAGES:');
    for (const message of [...new Set(uncompiled)]) console.log(' ', message);
    throw new Error(`${uncompiled.length} uncompiled message(s)`);
  }

  console.log('I18N CHECK PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
