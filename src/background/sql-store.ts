// The SQL side of the wasm bridge: a sql.js database owned by the service
// worker, persisted as an image into IndexedDB. Each wallet and network pair
// has its own database, mirroring how desktop stores one SQLite file per
// fingerprint and network. The schema comes from the Rust migrations
// (sage-database), never from TypeScript.
import initSqlJs, { Database, SqlJsStatic } from 'sql.js';

const IDB_NAME = 'sage_db';
const IDB_STORE = 'database';
const PERSIST_DEBOUNCE_MS = 500;

let sql: SqlJsStatic | null = null;
let db: Database | null = null;
let dbKey: string | null = null;
let persistTimer: ReturnType<typeof setTimeout> | null = null;
let persisting: Promise<void> = Promise.resolve();

export function databaseKey(fingerprint: number, networkId: string): string {
  return `${fingerprint}_${networkId}`;
}

type JsSqlValue =
  | { type: 'null' }
  | { type: 'int'; value: number }
  | { type: 'real'; value: number }
  | { type: 'text'; value: string }
  | { type: 'blob'; value: number[] };

function openIdb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(IDB_NAME, 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore(IDB_STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function loadImage(key: string): Promise<Uint8Array | null> {
  const idb = await openIdb();
  return new Promise((resolve, reject) => {
    const request = idb
      .transaction(IDB_STORE, 'readonly')
      .objectStore(IDB_STORE)
      .get(key);
    request.onsuccess = () =>
      resolve(request.result ? new Uint8Array(request.result) : null);
    request.onerror = () => reject(request.error);
  });
}

async function saveImage(key: string, image: Uint8Array): Promise<void> {
  const idb = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = idb.transaction(IDB_STORE, 'readwrite');
    tx.objectStore(IDB_STORE).put(image.buffer, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

function persistNow() {
  const key = dbKey;
  const database = db;

  if (!key || !database) return;

  const image = database.export();
  persisting = persisting.then(() => saveImage(key, image));
}

function schedulePersist() {
  if (persistTimer) {
    clearTimeout(persistTimer);
  }

  persistTimer = setTimeout(persistNow, PERSIST_DEBOUNCE_MS);
}

/** Waits until every scheduled persist has been written to IndexedDB. */
export async function flushDb(): Promise<void> {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
    persistNow();
  }
  await persisting;
}

function toBinding(value: JsSqlValue): number | string | Uint8Array | null {
  switch (value.type) {
    case 'null':
      return null;
    case 'int':
    case 'real':
      return value.value;
    case 'text':
      return value.value;
    case 'blob':
      return new Uint8Array(value.value);
  }
}

function fromResult(value: unknown): JsSqlValue {
  if (value === null || value === undefined) return { type: 'null' };
  if (value instanceof Uint8Array) return { type: 'blob', value: Array.from(value) };
  if (typeof value === 'number') {
    return Number.isInteger(value)
      ? { type: 'int', value }
      : { type: 'real', value };
  }
  return { type: 'text', value: String(value) };
}

// Splits multi-statement SQL and distributes bound parameters across the
// statements in order, matching sqlx's SQLite driver. Our queries never embed
// semicolons or question marks inside string literals.
function splitStatements(sql: string, params: JsSqlValue[]) {
  const statements = sql
    .split(';')
    .map((statement) => statement.trim())
    .filter(Boolean);

  let offset = 0;

  return statements.map((statement) => {
    const count = (statement.match(/\?/g) ?? []).length;
    const bindings = params.slice(offset, offset + count).map(toBinding);
    offset += count;
    return { statement, bindings };
  });
}

function required(): Database {
  if (!db) throw new Error('database is not initialized');
  return db;
}

export function registerSqlBridge() {
  const globals = globalThis as Record<string, unknown>;

  globals.dbQuery = (sql: string, paramsJson: string): string => {
    const database = required();
    const params: JsSqlValue[] = JSON.parse(paramsJson);
    const pieces = splitStatements(sql, params);

    let columns: string[] = [];
    let rows: JsSqlValue[][] = [];

    for (const { statement, bindings } of pieces) {
      const stmt = database.prepare(statement);
      try {
        stmt.bind(bindings);
        columns = stmt.getColumnNames();
        rows = [];
        while (stmt.step()) {
          rows.push(stmt.get().map(fromResult));
        }
      } finally {
        stmt.free();
      }
    }

    return JSON.stringify({ columns, rows });
  };

  globals.dbExecute = (sql: string, paramsJson: string): number => {
    const database = required();
    const params: JsSqlValue[] = JSON.parse(paramsJson);

    let changes = 0;

    for (const { statement, bindings } of splitStatements(sql, params)) {
      database.run(statement, bindings);
      changes = database.getRowsModified();
    }

    // Persisting exports the whole database, so it is always debounced: a
    // sync over a large wallet commits many times, and exporting on each one
    // stalls every other request. Durability comes from flushDb(), which the
    // worker awaits after each sync pass and before answering commands that
    // write.
    schedulePersist();

    return changes;
  };

  globals.dbExecuteBatch = (sql: string): void => {
    required().run(sql);
    schedulePersist();
  };

  // Used by the persistence benchmark to size the database image.
  globals.dbImageSize = (): number => required().export().length;
}

export async function initSqlEngine(): Promise<void> {
  if (sql) return;

  // sql.js falls back to XMLHttpRequest when given a file path, which doesn't
  // exist in service workers, so fetch the wasm binary ourselves.
  const wasmBinary = await (
    await fetch(chrome.runtime.getURL('wasm/sql-wasm.wasm'))
  ).arrayBuffer();

  sql = await initSqlJs({ wasmBinary } as never);
  registerSqlBridge();
}

/**
 * Removes every database belonging to a wallet, which is what deleting a key
 * does on desktop by removing the wallet's directory.
 */
export async function deleteDatabases(fingerprint: number): Promise<void> {
  const prefix = `${fingerprint}_`;

  if (dbKey?.startsWith(prefix)) {
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }

    await persisting;
    db?.close();
    db = null;
    dbKey = null;
  }

  const idb = await openIdb();

  await new Promise<void>((resolve, reject) => {
    const store = idb.transaction(IDB_STORE, 'readwrite').objectStore(IDB_STORE);
    const request = store.getAllKeys();

    request.onsuccess = () => {
      for (const key of request.result) {
        if (typeof key === 'string' && key.startsWith(prefix)) {
          store.delete(key);
        }
      }
    };
    request.onerror = () => reject(request.error);

    store.transaction.oncomplete = () => resolve();
    store.transaction.onerror = () => reject(store.transaction.error);
  });
}

/**
 * Makes the database for a wallet and network pair the active one, persisting
 * and closing the previous database first. Returns true when the database was
 * created empty, meaning migrations still have to run against it.
 */
export async function selectDatabase(key: string): Promise<boolean> {
  if (dbKey === key) return false;

  if (!sql) throw new Error('sql engine is not initialized');

  await flushDb();
  db?.close();

  const image = await loadImage(key);
  db = image ? new sql.Database(image) : new sql.Database();
  dbKey = key;

  return image === null;
}
