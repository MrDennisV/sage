// Shim for tauri-plugin-sage
// NFC/NDEF functionality not available in browser extensions

export async function isNdefAvailable(): Promise<boolean> {
  return false;
}

export async function getNdefPayloads(): Promise<number[][]> {
  return [];
}
