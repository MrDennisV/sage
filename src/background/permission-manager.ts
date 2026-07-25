// Which websites the user has connected the wallet to. The grants live in
// chrome.storage.local and are read back on every check rather than cached: a
// service worker is torn down and restarted constantly, and a stale in-memory
// copy would either resurrect a revoked grant or hide a fresh one.
const STORAGE_KEY = 'dappPermissions';

interface Grant {
  grantedAt: number;
}

type Grants = Record<string, Grant>;

async function readGrants(): Promise<Grants> {
  const stored = await chrome.storage.local.get(STORAGE_KEY);

  return (stored[STORAGE_KEY] as Grants | undefined) ?? {};
}

// Read-modify-write cycles from concurrent requests would lose grants, so they
// take turns.
let writes: Promise<unknown> = Promise.resolve();

function updateGrants(change: (grants: Grants) => Grants): Promise<void> {
  const run = writes.then(async () => {
    const grants = change(await readGrants());
    await chrome.storage.local.set({ [STORAGE_KEY]: grants });
  });

  writes = run.catch(() => {
    // A failed write must not block the next one.
  });

  return run;
}

export async function isOriginPermitted(origin: string): Promise<boolean> {
  if (!origin) return false;

  return origin in (await readGrants());
}

export function grantOrigin(origin: string): Promise<void> {
  return updateGrants((grants) => ({
    ...grants,
    [origin]: { grantedAt: Date.now() },
  }));
}

export function revokeOrigin(origin: string): Promise<void> {
  return updateGrants((grants) =>
    Object.fromEntries(
      Object.entries(grants).filter(([granted]) => granted !== origin),
    ),
  );
}
