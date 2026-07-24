// Shim for @tauri-apps/plugin-dialog

export interface SaveDialogOptions {
  defaultPath?: string;
  filters?: { name: string; extensions: string[] }[];
  title?: string;
}

export async function save(
  _options?: SaveDialogOptions,
): Promise<string | null> {
  // In browser extension, we can't use native file dialogs
  // The exportText function will need to use a download link instead
  return null;
}

export async function open(
  _options?: any,
): Promise<string | string[] | null> {
  return null;
}

export async function message(
  msg: string,
  _options?: any,
): Promise<boolean> {
  window.alert(msg);
  return true;
}

export async function ask(
  msg: string,
  _options?: any,
): Promise<boolean> {
  return window.confirm(msg);
}

export async function confirm(
  msg: string,
  _options?: any,
): Promise<boolean> {
  return window.confirm(msg);
}
