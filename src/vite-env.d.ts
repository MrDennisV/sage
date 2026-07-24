/// <reference types="vite/client" />

// True when built for the Chrome extension, where peer connections, the RPC
// server, and file logging don't exist.
declare const __IS_EXTENSION__: boolean;

// Allow importing JSON files as modules
declare module '*.json' {
  const value: never;
  export default value;
}
