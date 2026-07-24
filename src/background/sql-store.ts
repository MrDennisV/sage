// The SQL side of the wasm bridge: a sql.js database owned by the service
// worker, persisted as a single image into IndexedDB. The schema comes from
// the Rust migrations (sage-database), never from TypeScript.
import initSqlJs, { Database } from 'sql.js';

const IDB_NAME = 'sage_db';
const IDB_STORE = 'database';
const IDB_KEY = 'main';
const PERSIST_DEBOUNCE_MS = 500;

let db: Database | null = null;
let persistTimer: ReturnType<typeof setTimeout> | null = null;
let persisting: Promise<void> = Promise.resolve();

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

async function loadImage(): Promise<Uint8Array | null> {
  const idb = await openIdb();
  return new Promise((resolve, reject) => {
    const request = idb
      .transaction(IDB_STORE, 'readonly')
      .objectStore(IDB_STORE)
      .get(IDB_KEY);
    request.onsuccess = () =>
      resolve(request.result ? new Uint8Array(request.result) : null);
    request.onerror = () => reject(request.error);
  });
}

async function saveImage(image: Uint8Array): Promise<void> {
  const idb = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = idb.transaction(IDB_STORE, 'readwrite');
    tx.objectStore(IDB_STORE).put(image.buffer, IDB_KEY);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

function schedulePersist(immediate: boolean) {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }

  const run = () => {
    persisting = persisting.then(() => (db ? saveImage(db.export()) : undefined));
  };

  if (immediate) {
    run();
  } else {
    persistTimer = setTimeout(run, PERSIST_DEBOUNCE_MS);
  }
}

/** Waits until every scheduled persist has been written to IndexedDB. */
export async function flushDb(): Promise<void> {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
    persisting = persisting.then(() =>
      db ? saveImage(db.export()) : undefined,
    );
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

    const isCommit = sql.trim().toUpperCase().startsWith('COMMIT');
    schedulePersist(isCommit);

    return changes;
  };

  globals.dbExecuteBatch = (sql: string): void => {
    required().run(sql);
    schedulePersist(false);
  };
}

export async function initDatabase(): Promise<void> {
  if (db) return;

  const SQL = await initSqlJs({
    locateFile: (file) => `wasm/${file}`,
  });

  const image = await loadImage();
  db = image ? new SQL.Database(image) : new SQL.Database();
  registerSqlBridge();
}
