// Shim for @tauri-apps/plugin-barcode-scanner
// Barcode scanning not available in browser extensions

export enum Format {
  QRCode = 'QR_CODE',
  UPC_A = 'UPC_A',
  UPC_E = 'UPC_E',
  EAN_8 = 'EAN_8',
  EAN_13 = 'EAN_13',
  Code39 = 'CODE_39',
  Code93 = 'CODE_93',
  Code128 = 'CODE_128',
  Codabar = 'CODABAR',
  ITF = 'ITF',
  Aztec = 'AZTEC',
  DataMatrix = 'DATA_MATRIX',
  PDF417 = 'PDF_417',
}

export async function scan(
  _options?: { formats?: Format[] },
): Promise<{ content: string; format: string }> {
  throw new Error('Barcode scanning is not available in browser extensions');
}

export async function cancel(): Promise<void> {
  // No-op
}

export function checkPermissions(): Promise<{ camera: string }> {
  return Promise.resolve({ camera: 'denied' });
}

export async function requestPermissions(): Promise<string> {
  return 'denied';
}

export async function openAppSettings(): Promise<void> {
  // No-op in browser extension
}
