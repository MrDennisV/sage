// Post-build script: copies manifest.json, WASM files, and generates placeholder icons
import { copyFileSync, mkdirSync, existsSync, readdirSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');
const dist = resolve(root, 'dist-extension');

// Copy manifest.json
copyFileSync(resolve(root, 'manifest.json'), resolve(dist, 'manifest.json'));
console.log('Copied manifest.json');

// Copy WASM files
const wasmSrc = resolve(root, 'src/extension/wasm');
const wasmDist = resolve(dist, 'wasm');
mkdirSync(wasmDist, { recursive: true });

if (existsSync(wasmSrc)) {
  for (const file of readdirSync(wasmSrc)) {
    if (file.endsWith('.wasm') || file.endsWith('.js')) {
      copyFileSync(resolve(wasmSrc, file), resolve(wasmDist, file));
      console.log(`Copied wasm/${file}`);
    }
  }
}

// Copy sql.js WASM file (SQLite engine for the extension)
const sqlWasmSrc = resolve(root, 'node_modules/sql.js/dist/sql-wasm.wasm');
if (existsSync(sqlWasmSrc)) {
  copyFileSync(sqlWasmSrc, resolve(wasmDist, 'sql-wasm.wasm'));
  console.log('Copied wasm/sql-wasm.wasm (sql.js SQLite engine)');
}

// Copy Sage icons from Tauri build
mkdirSync(resolve(dist, 'icons'), { recursive: true });
const tauriIcons = resolve(root, 'src-tauri/icons');
const iconMap = { 16: 'Square30x30Logo.png', 48: 'Square44x44Logo.png', 128: '128x128.png' };
for (const [size, file] of Object.entries(iconMap)) {
  const src = resolve(tauriIcons, file);
  if (existsSync(src)) {
    copyFileSync(src, resolve(dist, `icons/icon-${size}.png`));
  }
}
console.log('Copied Sage icons');
console.log('Extension build complete! Load dist-extension/ in chrome://extensions');
