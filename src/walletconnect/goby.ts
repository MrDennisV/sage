// Goby (docs.goby.app) is the API real Chia dApps target, so `window.chia`
// speaks it. Its method names are bare where Sage's WalletConnect commands are
// namespaced, and a couple of its methods shape their params differently. This
// module is the single place that translates between the two vocabularies; the
// injected provider and the service worker both run requests through it so the
// mapping can never drift between them.
import type { WalletConnectCommand } from './commands';

/** Goby method names that have a Sage WalletConnect command behind them. */
const GOBY_COMMANDS: Record<string, WalletConnectCommand> = {
  connect: 'chip0002_connect',
  chainId: 'chip0002_chainId',
  getPublicKeys: 'chip0002_getPublicKeys',
  filterUnlockedCoins: 'chip0002_filterUnlockedCoins',
  getAssetCoins: 'chip0002_getAssetCoins',
  getAssetBalance: 'chip0002_getAssetBalance',
  signCoinSpends: 'chip0002_signCoinSpends',
  signMessage: 'chip0002_signMessage',
  sendTransaction: 'chip0002_sendTransaction',
  takeOffer: 'chia_takeOffer',
  createOffer: 'chia_createOffer',
  cancelOffer: 'chia_cancelOffer',
  transfer: 'chia_send',
  send: 'chia_send',
  getAddress: 'chia_getAddress',
  getNfts: 'chia_getNfts',
  signMessageByAddress: 'chia_signMessageByAddress',
  bulkMintNfts: 'chia_bulkMintNfts',
};

/**
 * Methods the browser extension answers itself because they are about the
 * connection between a site and the wallet rather than about the wallet's
 * contents, so there is no WalletConnect command for them.
 */
export const NATIVE_METHODS = [
  'disconnect',
  'walletWatchAsset',
  'walletSwitchChain',
] as const;

export type NativeMethod = (typeof NATIVE_METHODS)[number];

export interface DappRequest {
  method: WalletConnectCommand | NativeMethod;
  params: unknown;
}

/** Goby's `transfer` names the recipient `to`; Sage's send calls it `address`. */
function translateTransfer(params: unknown): unknown {
  if (typeof params !== 'object' || params === null) return params;

  const { to, ...rest } = params as Record<string, unknown> & { to?: unknown };

  return to === undefined ? params : { ...rest, address: to };
}

function isNativeMethod(method: string): method is NativeMethod {
  return (NATIVE_METHODS as readonly string[]).includes(method);
}

function isWalletConnectCommand(
  method: string,
): method is WalletConnectCommand {
  return method.startsWith('chip0002_') || method.startsWith('chia_');
}

/**
 * Normalises a request from a dApp into the vocabulary the wallet uses.
 * Already-namespaced methods and the extension's native methods pass through
 * untouched, so calling this twice is safe.
 */
export function toDappRequest(method: string, params?: unknown): DappRequest {
  if (isNativeMethod(method)) {
    return { method, params };
  }

  const command = GOBY_COMMANDS[method];

  if (command) {
    return {
      method: command,
      params: method === 'transfer' ? translateTransfer(params) : params,
    };
  }

  if (isWalletConnectCommand(method)) {
    return { method, params };
  }

  throw new Error(`Unsupported method: ${method}`);
}
