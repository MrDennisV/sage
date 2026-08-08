// Shim for @tauri-apps/plugin-opener

export async function openUrl(url: string): Promise<void> {
  window.open(url, '_blank');
}
