// Shim for tauri-plugin-safe-area-insets
// No safe area insets needed in browser extensions

export interface Insets {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export async function getInsets(): Promise<Insets> {
  return { top: 0, bottom: 0, left: 0, right: 0 };
}
