// Shim for @tauri-apps/plugin-biometric
// Biometric auth is not available in browser extensions

export async function authenticate(): Promise<void> {
  // No-op: biometric not available
}

export async function checkStatus(): Promise<{
  isAvailable: boolean;
  biometryType: number;
}> {
  return { isAvailable: false, biometryType: 0 };
}

export enum BiometryType {
  None = 0,
  TouchID = 1,
  FaceID = 2,
}

export interface AuthOptions {
  reason?: string;
  title?: string;
  subtitle?: string;
  description?: string;
  fallbackTitle?: string;
  cancelTitle?: string;
  allowDeviceCredential?: boolean;
  confirmationRequired?: boolean;
  maxAttempts?: number;
}

export interface Status {
  isAvailable: boolean;
  biometryType: BiometryType;
  error?: string;
  errorCode?: string;
}
