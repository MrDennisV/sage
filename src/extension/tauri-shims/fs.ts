// Shim for @tauri-apps/plugin-fs
// File system operations use download links in extension context

export async function writeTextFile(
  path: string,
  contents: string,
): Promise<void> {
  // In extension context, trigger a download instead
  const blob = new Blob([contents], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = path.split('/').pop() || 'download.txt';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export async function readTextFile(_path: string): Promise<string> {
  throw new Error('readTextFile is not available in extension context');
}

export async function exists(_path: string): Promise<boolean> {
  return false;
}

export async function mkdir(
  _path: string,
  _options?: any,
): Promise<void> {}

export async function remove(
  _path: string,
  _options?: any,
): Promise<void> {}
