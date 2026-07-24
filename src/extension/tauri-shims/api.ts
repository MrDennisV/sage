// Shim for @tauri-apps/api/core, @tauri-apps/api/event,
// @tauri-apps/api/window, @tauri-apps/api/app, @tauri-apps/api/webviewWindow

// --- @tauri-apps/api/core ---
export async function invoke(_cmd: string, _args?: any): Promise<any> {
  throw new Error('Tauri invoke not available in extension context');
}

export class Channel<T = unknown> {
  onmessage: ((message: T) => void) | undefined;
}

// --- @tauri-apps/api/window ---
export function getCurrentWindow() {
  return {
    requestUserAttention: (_type: number) => {},
    listen: (_event: string, _handler: any) => () => {},
    once: (_event: string, _handler: any) => () => {},
    emit: (_event: string, _payload?: any) => {},
  };
}

export const UserAttentionType = {
  Critical: 1,
  Informational: 2,
};

// --- @tauri-apps/api/app ---
export async function getVersion(): Promise<string> {
  return '0.1.0-extension';
}

// --- @tauri-apps/api/event ---
export type EventCallback<T> = (event: { payload: T }) => void;

export async function listen<T>(
  _event: string,
  _handler: EventCallback<T>,
): Promise<() => void> {
  return () => {};
}

export async function once<T>(
  _event: string,
  _handler: EventCallback<T>,
): Promise<() => void> {
  return () => {};
}

export async function emit(
  _event: string,
  _payload?: unknown,
): Promise<void> {}

// --- @tauri-apps/api/webviewWindow ---
export type WebviewWindow = ReturnType<typeof getCurrentWindow>;
