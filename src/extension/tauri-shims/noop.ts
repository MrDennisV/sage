// Generic no-op shim for unused Tauri plugins

// @buildyourwebapp/tauri-plugin-sharesheet
export async function shareText(_text: string): Promise<void> {
  // In extension context, copy to clipboard instead
  try {
    await navigator.clipboard.writeText(_text);
  } catch {
    // Silently fail if clipboard not available
  }
}

export default {};
