import { platform } from '@tauri-apps/plugin-os';

const currentPlatform = platform();

/**
 * Whether this build can host Sage apps. They run as child webviews of the
 * desktop window with storage isolated per app, which neither the mobile shells
 * nor the browser can provide.
 */
export const supportsSageApps =
  !__IS_EXTENSION__ &&
  currentPlatform !== 'android' &&
  currentPlatform !== 'ios';
