// The names a dApp calls, and what each one means to the wallet.
//
// Chia dApps are written against bare method names — `connect`, `getPublicKeys`
// — where Sage's WalletConnect commands are namespaced, and a couple of them
// shape their params differently. Goby (docs.goby.app) is where that vocabulary
// comes from, and matching it is what lets a site written for Goby work here
// unchanged. This module is the single place the two are reconciled; the
// injected provider and the service worker both run requests through it, so the
// mapping cannot drift between them.
import type { WalletConnectCommand } from './commands';

/** Bare method names that have a Sage WalletConnect command behind them. */
const DAPP_COMMANDS: Record<string, WalletConnectCommand> = {
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

  const command = DAPP_COMMANDS[method];

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
