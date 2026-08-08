// Minting an NFT without an explicit hash means downloading what the URI
// points at so the hash can be computed. That download used to be native only,
// which left the browser refusing to mint at all. The same fetcher fills in an
// NFT's name, description and picture after it arrives.
// Run with: node tests/extension/nft-uri-walk.mjs
import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { createServer } from 'http';
import { bech32m } from 'bech32';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const dist = resolve(root, 'dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-ext-'));

// A 1x1 PNG, served with the header that lets an extension read a cross-origin
// response. Hosting it here keeps the test off the public gateways.
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  'base64',
);

let served = false;

const server = createServer((_request, response) => {
  served = true;
  response.writeHead(200, {
    'content-type': 'image/png',
    'access-control-allow-origin': '*',
  });
  response.end(PNG);
});

await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
const uri = `http://127.0.0.1:${server.address().port}/pixel.png`;
console.log('serving', uri);

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
    name: 'NFT URI Test',
    key: mnemonic,
    save_secrets: true,
    login: true,
  });
  await check('login', { fingerprint });
  console.log('logged in, fingerprint:', fingerprint);

  // The wallet holds nothing, so this cannot get as far as building a spend.
  // What it proves is where it stops: reaching the funds means the content was
  // downloaded and hashed, which is the step that used to be refused outright.
  // Any well formed DID gets past the address check; the wallet does not own
  // it, which is fine because this never reaches the point of spending.
  const didId = bech32m.encode(
    'did:chia:',
    bech32m.toWords(Buffer.alloc(32, 1)),
    128,
  );

  const minted = await send('bulk_mint_nfts', {
    did_id: didId,
    mints: [{ data_uris: [uri] }],
    fee: '0',
    auto_submit: false,
  });

  const reason = minted?.error ?? '';
  console.log('mint stopped at:', reason || '(it succeeded)');

  if (reason.includes("can't be downloaded here")) {
    throw new Error('the browser still refuses to download URI content');
  }

  // A request the dispatch could not even read would stop before the download
  // and look just like success here.
  if (
    reason.includes('missing field') ||
    reason.includes('unsupported command') ||
    reason.includes('Address error')
  ) {
    throw new Error(`the mint request never reached the download: ${reason}`);
  }

  if (!served) {
    throw new Error('the worker never asked for the content');
  }

  // Running out of coins is as far as an empty wallet can get, and it is past
  // the download: the content was fetched and hashed to build the mint.
  if (!reason.includes('no spendable coins')) {
    throw new Error(`the mint stopped somewhere unexpected: ${reason}`);
  }

  console.log('NFT URI WALK PASSED');
} finally {
  await context.close();
  rmSync(profile, { recursive: true, force: true });
  server.close();
}
