import react from '@vitejs/plugin-react-swc';
import { lingui } from '@lingui/vite-plugin';
import path from 'path';
import { defineConfig, Plugin } from 'vite';

// Plugin to redirect ALL imports that resolve to src/bindings.ts
// to our extension adapter, regardless of whether they use
// '@/bindings', '../bindings', or './bindings'
function bindingsRedirectPlugin(): Plugin {
  const bindingsPath = path
    .resolve(__dirname, 'src/bindings.ts')
    .replace(/\\/g, '/');
  const adapterPath = path
    .resolve(__dirname, 'src/extension/bindings-adapter.ts')
    .replace(/\\/g, '/');

  return {
    name: 'bindings-redirect',
    enforce: 'pre',
    resolveId(source, importer) {
      if (!importer) return null;

      // NEVER redirect imports FROM the adapter itself (it needs the original for types)
      const importerNorm = importer.replace(/\\/g, '/');
      if (importerNorm === adapterPath) {
        return null;
      }

      // Check if this import would resolve to src/bindings.ts
      if (source === '@/bindings') {
        return adapterPath;
      }

      // Handle relative imports like '../bindings' or './bindings'
      if (
        (source.endsWith('/bindings') ||
          source === './bindings' ||
          source === '../bindings') &&
        !source.includes('bindings-adapter')
      ) {
        const importerDir = path.dirname(importer).replace(/\\/g, '/');
        const resolved = path
          .resolve(importerDir, source + '.ts')
          .replace(/\\/g, '/');
        if (resolved === bindingsPath) {
          return adapterPath;
        }
      }

      return null;
    },
  };
}

export default defineConfig({
  plugins: [
    bindingsRedirectPlugin(),
    react({
      plugins: [['@lingui/swc-plugin', {}]],
    }),
    lingui(),
  ],
  base: './',
  build: {
    rollupOptions: {
      input: {
        popup: path.resolve(__dirname, 'popup.html'),
        'service-worker': path.resolve(
          __dirname,
          'src/background/service-worker.ts',
        ),
        'content-bridge': path.resolve(
          __dirname,
          'src/content/content-bridge.ts',
        ),
        inject: path.resolve(__dirname, 'src/content/inject.ts'),
      },
      output: {
        entryFileNames: '[name].js',
        chunkFileNames: 'chunks/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
    outDir: 'dist-extension',
    chunkSizeWarningLimit: 2048,
  },
  resolve: {
    alias: {
      // Redirect @tauri-apps/* to browser shims
      '@tauri-apps/api/core': path.resolve(
        __dirname,
        './src/extension/tauri-shims/api',
      ),
      '@tauri-apps/api/event': path.resolve(
        __dirname,
        './src/extension/tauri-shims/api',
      ),
      '@tauri-apps/api/window': path.resolve(
        __dirname,
        './src/extension/tauri-shims/api',
      ),
      '@tauri-apps/api/app': path.resolve(
        __dirname,
        './src/extension/tauri-shims/api',
      ),
      '@tauri-apps/api/webviewWindow': path.resolve(
        __dirname,
        './src/extension/tauri-shims/api',
      ),
      '@tauri-apps/plugin-clipboard-manager': path.resolve(
        __dirname,
        './src/extension/tauri-shims/clipboard',
      ),
      '@tauri-apps/plugin-opener': path.resolve(
        __dirname,
        './src/extension/tauri-shims/opener',
      ),
      '@tauri-apps/plugin-os': path.resolve(
        __dirname,
        './src/extension/tauri-shims/os',
      ),
      '@tauri-apps/plugin-biometric': path.resolve(
        __dirname,
        './src/extension/tauri-shims/biometric',
      ),
      '@tauri-apps/plugin-barcode-scanner': path.resolve(
        __dirname,
        './src/extension/tauri-shims/barcode-scanner',
      ),
      '@tauri-apps/plugin-dialog': path.resolve(
        __dirname,
        './src/extension/tauri-shims/dialog',
      ),
      '@tauri-apps/plugin-fs': path.resolve(
        __dirname,
        './src/extension/tauri-shims/fs',
      ),
      '@buildyourwebapp/tauri-plugin-sharesheet': path.resolve(
        __dirname,
        './src/extension/tauri-shims/noop',
      ),
      'tauri-plugin-safe-area-insets': path.resolve(
        __dirname,
        './src/extension/tauri-shims/safe-area',
      ),
      'tauri-plugin-sage': path.resolve(
        __dirname,
        './src/extension/tauri-shims/sage-plugin',
      ),

      // Keep the original @ alias
      '@': path.resolve(__dirname, './src'),
    },
  },
  define: {
    __IS_EXTENSION__: true,
  },
});
