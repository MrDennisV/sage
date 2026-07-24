// Shim for @tauri-apps/plugin-os

export async function platform(): Promise<string> {
  return 'web';
}

export async function arch(): Promise<string> {
  return 'wasm';
}

export async function type(): Promise<string> {
  return 'Browser';
}

export async function version(): Promise<string> {
  return navigator.userAgent;
}
