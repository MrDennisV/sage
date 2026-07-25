// Guards the seam between the interface and the wasm core: every command the
// bindings adapter can invoke has to be dispatchable, or be listed below as
// deliberately absent. Pure static analysis, so it runs without a browser.
import { readFileSync } from 'fs';
import { resolve } from 'path';

const adapterPath = resolve('src/extension/bindings-adapter.ts');
const dispatchPath = resolve('crates/sage-wasm/src/dispatch.rs');

// Commands the browser build will never answer. Each one needs a reason that
// explains what the browser is missing, not just that it is unimplemented.
const INTENTIONALLY_UNSUPPORTED = new Map([
  // There is no peer connection pool in the browser; every request goes
  // straight to the Coinset HTTP API.
  ['add_peer', 'no peer connections'],
  ['remove_peer', 'no peer connections'],
  ['set_discover_peers', 'no peer connections'],
  ['set_target_peers', 'no peer connections'],
  // Logs are rotated files written by tracing-appender, which needs a disk.
  ['get_logs', 'no log files'],
  // Both report or act on SQLite file statistics (page counts, WAL, VACUUM),
  // which the in-memory sql.js database does not have.
  ['get_database_stats', 'SQLite file statistics'],
  ['perform_database_maintenance', 'SQLite file statistics'],
]);

function read(path) {
  return readFileSync(path, 'utf8');
}

/** Every command name the interface can reach through the adapter. */
function invokedCommands(source) {
  const names = new Set();

  for (const match of source.matchAll(/invoke(?:<[^>]*>)?\(\s*'([a-z0-9_]+)'/g)) {
    names.add(match[1]);
  }

  return names;
}

/** The endpoint names listed in the `impl_endpoints_portable!` macro call. */
function generatedCommands(source) {
  const start = source.indexOf('impl_endpoints_portable!');

  if (start === -1) {
    throw new Error('no impl_endpoints_portable! call in dispatch.rs');
  }

  const open = source.indexOf('(', start);
  const close = source.indexOf(')', open);

  if (open === -1 || close === -1) {
    throw new Error('malformed impl_endpoints_portable! endpoint list');
  }

  return new Set(
    source
      .slice(open + 1, close)
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean),
  );
}

/** The command strings matched by the hand-written dispatch arms. */
function handWrittenCommands(source) {
  const names = new Set();
  const arms = /^\s*("[a-z0-9_]+"(?:\s*\|\s*"[a-z0-9_]+")*)\s*=>/gm;

  for (const arm of source.matchAll(arms)) {
    for (const name of arm[1].matchAll(/"([a-z0-9_]+)"/g)) {
      names.add(name[1]);
    }
  }

  return names;
}

const adapter = read(adapterPath);
const dispatch = read(dispatchPath);

const invoked = invokedCommands(adapter);
const supported = new Set([
  ...generatedCommands(dispatch),
  ...handWrittenCommands(dispatch),
]);

if (invoked.size === 0 || supported.size === 0) {
  throw new Error('extraction found nothing; the source layout changed');
}

const missing = [...invoked]
  .filter((cmd) => !supported.has(cmd))
  .filter((cmd) => !INTENTIONALLY_UNSUPPORTED.has(cmd))
  .sort();

// An allowlist entry that the dispatch has since grown is stale, and hides the
// next real gap behind it.
const stale = [...INTENTIONALLY_UNSUPPORTED.keys()]
  .filter((cmd) => supported.has(cmd) || !invoked.has(cmd))
  .sort();

console.log(
  `${invoked.size} command(s) reachable, ${supported.size} dispatchable, ` +
    `${INTENTIONALLY_UNSUPPORTED.size} allowlisted`,
);

if (missing.length > 0) {
  console.error(
    `unsupported command(s) the interface can invoke:\n  ${missing.join('\n  ')}`,
  );
}

if (stale.length > 0) {
  console.error(
    `stale allowlist entr(ies), now dispatchable or unreachable:\n  ${stale.join('\n  ')}`,
  );
}

if (missing.length > 0 || stale.length > 0) {
  process.exit(1);
}

console.log('COMMAND AUDIT PASSED');
