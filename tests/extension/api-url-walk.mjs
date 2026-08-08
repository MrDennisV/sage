// Checks that the configurable API endpoint round-trips: setting a custom URL
// persists it, and the sync then talks to that host instead of the default.
import { chromium } from 'playwright';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-api-'));

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
          chrome.runtime.sendMessage(
            { type: 'COMMAND', cmd: c, args: { req: r } },
            res,
          ),
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

  const before = await check('get_networks');
  console.log(
    'defaults:',
    before.networks
      .map((n) => `${n.name}=${n.api_url ?? '(default)'}`)
      .join(', '),
  );

  await check('set_network_api_url', {
    name: 'mainnet',
    api_url: 'https://example.invalid/api',
  });

  const after = await check('get_networks');
  const mainnet = after.networks.find((n) => n.name === 'mainnet');

  if (mainnet.api_url !== 'https://example.invalid/api') {
    throw new Error(`api_url did not persist: ${JSON.stringify(mainnet)}`);
  }
  console.log('custom url saved:', mainnet.api_url);

  // The sync should now fail against the bogus host, proving it is used.
  const { data: mnemonic } = await send('generate_mnemonic', {
    use_24_words: true,
  });
  const { data: imported } = await send('import_key', {
    name: 'Api',
    key: mnemonic.mnemonic,
    save_secrets: true,
    login: true,
  });

  const errors = [];
  worker.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });

  await send('login', { fingerprint: imported.fingerprint });
  await page.waitForTimeout(20000);

  // The configured host is unreachable, so a request failure proves the sync
  // used it; against the default endpoint the sync succeeds.
  const usedCustomHost = errors.some((e) =>
    e.includes('error sending request'),
  );
  console.log(
    usedCustomHost
      ? 'sync failed against the configured endpoint, as expected'
      : `sync did not fail: ${errors.join(' | ') || 'no errors'}`,
  );

  // Restore the default so the check leaves nothing behind.
  await check('set_network_api_url', { name: 'mainnet', api_url: null });
  const restored = await check('get_networks');
  const reset = restored.networks.find((n) => n.name === 'mainnet');

  if (reset.api_url) {
    throw new Error(`api_url did not reset: ${reset.api_url}`);
  }
  console.log('reset to default');

  if (!usedCustomHost) {
    throw new Error('the configured endpoint was not used by the sync');
  }

  console.log('API URL TEST PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
