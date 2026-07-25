// Measures what a full database export costs as the wallet grows, which is
// what the persistence strategy has to stay ahead of.
import { chromium } from 'playwright';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-bench-'));

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium',
  headless: true,
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

try {
  let worker = context.serviceWorkers()[0];
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });

  // A database is only selected once a wallet is logged in.
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

  const { data: mnemonic } = await send('generate_mnemonic', { use_24_words: true });
  const { data: imported } = await send('import_key', {
    name: 'Bench',
    key: mnemonic.mnemonic,
    save_secrets: true,
    login: true,
  });
  await send('login', { fingerprint: imported.fingerprint });
  await page.waitForTimeout(3000);

  const results = await worker.evaluate(async () => {
    const out = [];

    for (const total of [500, 2000, 6000]) {
      globalThis.dbExecuteBatch(
        `CREATE TABLE IF NOT EXISTS bench (id INTEGER PRIMARY KEY, blob BLOB)`,
      );

      // Rows sized like a coin row, so the image grows realistically.
      for (let i = 0; i < total; i++) {
        globalThis.dbExecute(
          'INSERT INTO bench (blob) VALUES (?)',
          JSON.stringify([{ type: 'blob', value: Array(200).fill(7) }]),
        );
      }

      const started = performance.now();
      const image = globalThis.dbImageSize();
      const ms = performance.now() - started;

      out.push({ rows: total, imageMb: +(image / 1024 / 1024).toFixed(2), exportMs: Math.round(ms) });
    }

    return out;
  });

  console.log('rows inserted | image size | full export cost');
  for (const r of results) {
    console.log(`${String(r.rows).padStart(6)} | ${String(r.imageMb).padStart(6)} MB | ${r.exportMs} ms`);
  }
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
}
