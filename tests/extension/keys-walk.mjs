// Exercises the key lifecycle the way the interface does: create, list, wipe
// the wallet's data, delete, and confirm that a failure comes back with a
// readable message.
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

  // Derivations are the cheapest proof that the wallet still works: nothing
  // creates them but a sync, so they disappearing and coming back shows the
  // database was really emptied and really refilled.
  const derivations = async () =>
    (await check('get_derivations', { hardened: false, offset: 0, limit: 5 }))
      .total;

  const waitForDerivations = async (label) => {
    for (let attempt = 0; attempt < 60; attempt++) {
      const total = await derivations();
      if (total > 0) return total;
      await page.waitForTimeout(1000);
    }
    throw new Error(`derivations were never regenerated after ${label}`);
  };

  console.log(`synced ${await waitForDerivations('login')} derivation(s)`);

  await check('resync', {
    fingerprint,
    delete_coins: true,
    delete_assets: true,
    delete_files: true,
    delete_offers: true,
    delete_addresses: true,
    delete_blocks: true,
  });

  console.log(`resynced, ${await waitForDerivations('resync')} derivation(s)`);

  // Resyncing is offered from the wallet list, so it has to work on a wallet
  // the user is not signed into. Only one database is mounted at a time, so
  // the named wallet's takes its place for the command.
  const second = await check('generate_mnemonic', { use_24_words: true });
  const other = await check('import_key', {
    name: 'Keys Test Second',
    key: second.mnemonic,
    save_secrets: true,
    login: false,
  });

  await check('login', { fingerprint });
  await check('resync', {
    fingerprint: other.fingerprint,
    delete_coins: true,
    delete_blocks: true,
    delete_addresses: true,
  });

  const stillActive = await check('get_key', {});

  if (stillActive.key?.fingerprint !== fingerprint) {
    throw new Error(
      `resyncing another wallet left ${stillActive.key?.fingerprint} signed in`,
    );
  }

  // Clearing the other wallet's addresses must not have reached this one's,
  // which is what a resync run against the mounted database would have done.
  const kept = await check('get_derivations', { offset: 0, limit: 1 });

  if (kept.derivations.length === 0) {
    throw new Error('resyncing another wallet cleared this one instead');
  }
  console.log(`resynced wallet ${other.fingerprint}, this one untouched`);

  // A fingerprint no key answers to has to be refused, rather than quietly
  // building a database for a wallet that does not exist.
  const unknown = fingerprint > 1 ? fingerprint - 1 : fingerprint + 1;
  const refused = await send('resync', { fingerprint: unknown });

  if (!refused?.error) {
    throw new Error('resyncing an unknown wallet unexpectedly succeeded');
  }
  console.log(`unknown wallet refused: ${refused.error}`);

  await check('delete_key', { fingerprint: other.fingerprint });

  const { key } = await check('get_key', { fingerprint });
  await check('delete_database', { fingerprint, network: key.network_id });

  console.log(
    `database emptied, ${await waitForDerivations('delete_database')} derivation(s)`,
  );

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
