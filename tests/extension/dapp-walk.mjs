// Drives window.chia from a real web page the way a dApp would: everything
// goes through request({ method, params }), the approval popup is clicked by
// hand, and the results are checked. Run with: node tests/extension/dapp-walk.mjs
import { chromium } from 'playwright';
import { createServer } from 'http';
import { mkdtempSync, readFileSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';

const dist = resolve('dist-extension');
const profile = mkdtempSync(join(tmpdir(), 'sage-dapp-'));
const harness = readFileSync(resolve('dapp-test.html'));

// A dApp has to be served over http for content scripts to run.
const server = createServer((_request, response) => {
  response.writeHead(200, { 'content-type': 'text/html' });
  response.end(harness);
});

await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
const origin = `http://127.0.0.1:${server.address().port}`;

const context = await chromium.launchPersistentContext(profile, {
  channel: 'chromium',
  headless: true,
  args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
});

const results = [];
const failures = [];

function check(name, condition, detail) {
  results.push(
    `${condition ? 'PASS' : 'FAIL'} ${name}${detail ? ` :: ${detail}` : ''}`,
  );
  if (!condition) failures.push(`${name}${detail ? ` :: ${detail}` : ''}`);
}

/** Calls the provider and reports the outcome instead of throwing across the boundary. */
const call = (page, method, params) =>
  page.evaluate(
    ([m, p]) =>
      window.chia
        .request({ method: m, params: p })
        .then((result) => ({ ok: true, result }))
        .catch((error) => ({ ok: false, error: error.message })),
    [method, params],
  );

try {
  let worker = context.serviceWorkers()[0];
  worker ??= await context.waitForEvent('serviceworker', { timeout: 20000 });
  worker.on('console', (m) => {
    if (m.type() === 'error') failures.push(`[worker] ${m.text()}`);
  });

  const id = new URL(worker.url()).host;

  // ─── Import a wallet through the popup ───────────────────────────
  // The popup stays open: Playwright cannot reach into a toolbar popup, so the
  // approval dialog has to be answered in a normal tab of the same page. That
  // the request opens the popup by itself is checked separately at the end.
  const wallet = await context.newPage();
  await wallet.goto(`chrome-extension://${id}/popup.html`);
  await wallet.waitForSelector('#root > *', { timeout: 20000 });

  const send = (cmd, req = {}) =>
    wallet.evaluate(
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
    name: 'Dapp',
    key: mnemonic.mnemonic,
    save_secrets: true,
    login: true,
  });
  await send('login', { fingerprint: imported.fingerprint });

  // ─── The dApp page ───────────────────────────────────────────────
  const page = await context.newPage();
  page.on('console', (m) => {
    if (m.type() === 'error') failures.push(`[page] ${m.text()}`);
  });
  await page.goto(origin);
  await page.waitForFunction(() => typeof window.chia !== 'undefined', {
    timeout: 20000,
  });

  const provider = await page.evaluate(() => ({
    isSage: window.chia.isSage,
    isGoby: window.chia.isGoby,
    request: typeof window.chia.request,
    aliased: window.chia === window.sage,
  }));

  check(
    'window.chia is installed',
    provider.request === 'function',
    `request=${provider.request}`,
  );
  check('provider identifies as Sage', provider.isSage === true);
  check('provider does not claim to be Goby', provider.isGoby === undefined);
  check('window.sage aliases window.chia', provider.aliased === true);

  // ─── Before connecting ───────────────────────────────────────────
  const eagerBefore = await call(page, 'connect', { eager: true });
  check(
    'eager connect resolves false when not permitted',
    eagerBefore.ok && eagerBefore.result === false,
    JSON.stringify(eagerBefore),
  );

  const chainBefore = await call(page, 'chainId');
  check(
    'chainId is refused before connecting',
    !chainBefore.ok && /not connected/i.test(chainBefore.error ?? ''),
    JSON.stringify(chainBefore),
  );

  const unknown = await call(page, 'notAMethod');
  check(
    'unknown methods are refused',
    !unknown.ok && /unsupported method/i.test(unknown.error ?? ''),
    JSON.stringify(unknown),
  );

  // ─── connect() waits for the user to answer Sage's own dialog ────
  const connecting = call(page, 'connect');

  const dialog = wallet.getByRole('dialog');
  await dialog.waitFor({ timeout: 20000 });
  const dialogText = await dialog.innerText();

  check(
    'the approval dialog names the requesting origin',
    dialogText.includes(origin),
    dialogText.split('\n')[0],
  );

  await wallet.getByRole('button', { name: 'Approve' }).click();

  const connected = await connecting;
  check(
    'connect resolves true after approval',
    connected.ok && connected.result === true,
    JSON.stringify(connected),
  );

  // ─── Read-only methods now run without a prompt ──────────────────
  const chainId = await call(page, 'chainId');
  check(
    'chainId returns the active network',
    chainId.ok &&
      typeof chainId.result === 'string' &&
      chainId.result.length > 0,
    JSON.stringify(chainId),
  );

  const keys = await call(page, 'getPublicKeys', { limit: 3 });
  check(
    'getPublicKeys returns derived keys',
    keys.ok && Array.isArray(keys.result) && keys.result.length === 3,
    JSON.stringify(keys).slice(0, 200),
  );
  check(
    'public keys are hex encoded',
    keys.ok && keys.result.every((key) => /^(0x)?[0-9a-f]{96}$/i.test(key)),
    JSON.stringify(keys.result?.[0]),
  );

  const address = await call(page, 'getAddress');
  check(
    'getAddress returns a bech32m address',
    address.ok && /^(xch|txch)1[a-z0-9]+$/.test(address.result?.address ?? ''),
    JSON.stringify(address),
  );

  const balance = await call(page, 'getAssetBalance', {
    type: null,
    assetId: null,
  });
  check(
    'getAssetBalance returns a balance',
    balance.ok && typeof balance.result?.confirmed === 'string',
    JSON.stringify(balance),
  );

  // Coin ids are unprefixed hex, the same shape getAssetCoins hands back.
  const unlocked = await call(page, 'filterUnlockedCoins', {
    coinNames: ['11'.repeat(32)],
  });
  check(
    'filterUnlockedCoins answers with the spendable subset',
    unlocked.ok && Array.isArray(unlocked.result?.coin_ids),
    JSON.stringify(unlocked),
  );

  const eagerAfter = await call(page, 'connect', { eager: true });
  check(
    'eager connect resolves true once permitted',
    eagerAfter.ok && eagerAfter.result === true,
    JSON.stringify(eagerAfter),
  );

  // ─── A signing request runs Sage's own per-command dialog ────────
  const signing = call(page, 'signMessage', {
    message: 'Hello Sage!',
    publicKey: keys.result[0],
  });

  const signDialog = wallet.getByRole('dialog');
  await signDialog.waitFor({ timeout: 20000 });
  const signText = await signDialog.innerText();

  check(
    'signMessage shows the Sign Message dialog',
    signText.includes('Sign Message') && signText.includes(keys.result[0]),
    signText.split('\n').slice(0, 3).join(' | '),
  );

  await wallet.getByRole('button', { name: 'Approve' }).click();

  const signature = await signing;
  check(
    'signMessage returns a signature',
    signature.ok && /^[0-9a-f]{192}$/i.test(signature.result ?? ''),
    JSON.stringify(signature).slice(0, 160),
  );

  // ─── Rejecting sends the dApp an error, not a value ──────────────
  const rejected = call(page, 'signMessage', {
    message: 'Nope',
    publicKey: keys.result[0],
  });

  await wallet.getByRole('dialog').waitFor({ timeout: 20000 });
  await wallet.getByRole('button', { name: 'Reject' }).click();

  const refusal = await rejected;
  check(
    'rejecting a request fails the dApp call',
    refusal.ok === false && /reject/i.test(refusal.error ?? ''),
    JSON.stringify(refusal),
  );

  // ─── sendTransaction reaches the chain ───────────────────────────
  // A bundle with no spends is well formed and certain to be refused, so what
  // it proves is that the wallet carried it to the node and brought the answer
  // back: a browser build with no route to the chain could only refuse it
  // itself. CHIP-0002 reports a rejection in the response, not as an error.
  const pushed = await call(page, 'sendTransaction', {
    spendBundle: {
      coin_spends: [],
      // The G2 point at infinity: a valid encoding, so the wallet gets past
      // parsing and the node is the one that decides.
      aggregated_signature: `0xc0${'0'.repeat(190)}`,
    },
  });

  check(
    'sendTransaction reaches the node and reports its answer',
    pushed.ok &&
      typeof pushed.result?.status === 'number' &&
      pushed.result.status !== 1,
    JSON.stringify(pushed).slice(0, 200),
  );

  check(
    'sendTransaction is not refused as unsupported',
    !/unsupported/i.test(JSON.stringify(pushed)),
    JSON.stringify(pushed).slice(0, 200),
  );

  // ─── A wallet change reaches the page as accountChanged ─────────
  await send('login', { fingerprint: imported.fingerprint });
  await page
    .waitForFunction(() => window.sageTestEvents?.includes('accountChanged'), {
      timeout: 15000,
    })
    .catch(() => {});

  check(
    'accountChanged reached the page',
    (await page.evaluate(() => window.sageTestEvents ?? [])).includes(
      'accountChanged',
    ),
  );

  // ─── walletSwitchChain moves the wallet and tells the page ───────
  const target = chainId.result === 'mainnet' ? 'testnet11' : 'mainnet';
  const switched = await call(page, 'walletSwitchChain', { chainId: target });
  check(
    `walletSwitchChain to ${target}`,
    switched.ok && switched.result === null,
    JSON.stringify(switched),
  );

  const afterSwitch = await call(page, 'chainId');
  check(
    'chainId reflects the switch',
    afterSwitch.ok && afterSwitch.result === target,
    JSON.stringify(afterSwitch),
  );

  await page
    .waitForFunction(() => window.sageTestEvents?.includes('chainChanged'), {
      timeout: 15000,
    })
    .catch(() => {});

  const events = await page.evaluate(() => window.sageTestEvents ?? []);
  check(
    'chainChanged reached the page',
    events.includes('chainChanged'),
    JSON.stringify(events),
  );

  // ─── Unsupported shapes fail loudly instead of faking success ────
  const watch = await call(page, 'walletWatchAsset', {
    type: 'CAT',
    options: { assetId: 'a'.repeat(64), symbol: 'NOPE' },
  });
  check(
    'walletWatchAsset refuses an asset the wallet does not know',
    !watch.ok && /not known/i.test(watch.error ?? ''),
    JSON.stringify(watch),
  );

  // ─── disconnect revokes the grant ────────────────────────────────
  const disconnected = await call(page, 'disconnect');
  check('disconnect resolves', disconnected.ok, JSON.stringify(disconnected));

  const chainAfterDisconnect = await call(page, 'chainId');
  check(
    'chainId is refused again after disconnect',
    !chainAfterDisconnect.ok,
    JSON.stringify(chainAfterDisconnect),
  );

  // ─── Walking away from the wallet fails the call, never hangs ────
  const abandoning = call(page, 'connect');
  await wallet.getByRole('dialog').waitFor({ timeout: 20000 });
  await wallet.close();

  const abandoned = await Promise.race([
    abandoning,
    page.waitForTimeout(20000).then(() => ({ ok: 'timeout' })),
  ]);

  check(
    'closing the wallet rejects the request instead of hanging',
    abandoned.ok === false,
    JSON.stringify(abandoned),
  );

  // ─── With no wallet window open, the request opens the popup ─────
  // Playwright cannot see into a toolbar popup, so ask the service worker.
  const reconnecting = call(page, 'connect');

  let opened = [];
  for (let attempt = 0; attempt < 30 && opened.length === 0; attempt++) {
    await page.waitForTimeout(500);
    opened = await worker.evaluate(() =>
      chrome.runtime.getContexts({ contextTypes: ['POPUP'] }),
    );
  }

  check(
    'connect opens the extension popup',
    opened.length > 0 && opened[0].documentUrl.includes('popup.html'),
    JSON.stringify(opened.map((entry) => entry.documentUrl)),
  );

  // Nothing will answer it; make sure the harness does not wait on it.
  reconnecting.catch(() => {});
} catch (error) {
  failures.push(`[harness] ${error?.stack ?? error}`);
} finally {
  console.log(results.join('\n'));
  console.log('\n=== failures ===');
  console.log(failures.length ? [...new Set(failures)].join('\n') : 'none');

  await context.close();
  server.close();
  rmSync(profile, { recursive: true, force: true });

  if (failures.length) process.exitCode = 1;
}
