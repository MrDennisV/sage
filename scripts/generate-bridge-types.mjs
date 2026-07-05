import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

execFileSync(
  'cargo',
  ['run', '-p', 'sage-apps', '--bin', 'export_bridge_types'],
  {
    cwd: repoRoot,
    stdio: ['ignore', 'ignore', 'inherit'],
  },
);
