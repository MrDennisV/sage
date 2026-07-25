// The popup reaches the wallet by messaging the service worker. The service
// worker runs the very same command handlers when it answers read-only dApp
// requests, and messaging itself would deadlock, so it installs a dispatcher
// that calls the wasm core directly. Whichever side is loaded, `commands` in
// the bindings adapter routes through here.
export type CommandDispatcher = (
  cmd: string,
  request: unknown,
) => Promise<unknown>;

let dispatcher: CommandDispatcher | null = null;

export function setCommandDispatcher(next: CommandDispatcher | null): void {
  dispatcher = next;
}

export function commandDispatcher(): CommandDispatcher | null {
  return dispatcher;
}
