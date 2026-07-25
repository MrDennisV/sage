// The confirmation UI for WalletConnect commands, shared by every transport
// that can deliver one: the WalletConnect relay on desktop and the dApp bridge
// in the browser extension. The dialog only knows about a command, its params
// and who is asking; approving and rejecting are the caller's job.
import {
  commands,
  OfferRecord,
  OfferSummary,
  TransactionSummary,
} from '@/bindings';
import { AdvancedTransactionSummary } from '@/components/AdvancedTransactionSummary';
import { OfferCard } from '@/components/OfferCard';
import { OfferSummaryCard } from '@/components/OfferSummaryCard';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { LoadingButton } from '@/components/ui/loading-button';
import { Switch } from '@/components/ui/switch';
import { useErrors } from '@/hooks/useErrors';
import { decodeHexMessage, formatAddress, fromMojos, isHex } from '@/lib/utils';
import { useWallet } from '@/contexts/WalletContext';
import { useWalletState } from '@/state';
import {
  Params,
  WalletConnectCommand,
  walletConnectCommands,
} from '@/walletconnect/commands';
import { t } from '@lingui/core/macro';
import { Trans } from '@lingui/react/macro';
import { AlertTriangleIcon, CheckCircle2Icon } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTheme } from 'theme-o-rama';
import { formatNumber } from '../i18n';

export interface RequestDialogProps {
  method: WalletConnectCommand;
  params: unknown;
  /** Name of the dApp or peer the request came from, if known. */
  peerName?: string | null;
  approve: () => Promise<void> | void;
  reject: () => void;
}

interface CommandDialogProps<T extends WalletConnectCommand> {
  params: Partial<Params<T>>;
  /** Where the request came from, for the dialogs that present it themselves. */
  peerName?: string | null;
}

/**
 * Takes over the extension popup. Set as styles rather than classes because
 * the dialog is centered and size-capped by its own utilities, which would
 * otherwise have to be unpicked one by one.
 */
const fullPopup: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  width: '100vw',
  height: '100vh',
  maxWidth: 'none',
  maxHeight: 'none',
  transform: 'none',
  borderRadius: 0,
  overflow: 'hidden',
};

function SignCoinSpendsDialog({
  params,
}: CommandDialogProps<'chip0002_signCoinSpends'>) {
  const [summary, setSummary] = useState<TransactionSummary | null>(null);
  const { addError } = useErrors();

  useEffect(() => {
    const coinSpends =
      params.coinSpends?.map((coinSpend) => ({
        coin: {
          parent_coin_info: coinSpend.coin.parent_coin_info,
          puzzle_hash: coinSpend.coin.puzzle_hash,
          amount: coinSpend.coin.amount.toString(),
        },
        puzzle_reveal: coinSpend.puzzle_reveal,
        solution: coinSpend.solution,
      })) ?? [];

    commands
      .viewCoinSpends({ coin_spends: coinSpends })
      .then((data) => setSummary(data.summary))
      .catch(addError);
  }, [params, addError]);

  return summary ? (
    <AdvancedTransactionSummary summary={summary} />
  ) : (
    <div className='p-4 text-center'>Loading transaction summary...</div>
  );
}

function MessageToSign(params: { message: string }) {
  const [showDecoded, setShowDecoded] = useState(false);
  const isHexMessage = isHex(params.message);
  const message = isHexMessage
    ? !showDecoded
      ? params.message
      : decodeHexMessage(params.message)
    : params.message;

  return (
    <div className='space-y-2'>
      <div className='flex items-center justify-between gap-2 flex-wrap'>
        <div className='font-medium'>
          Message{' '}
          {isHexMessage && showDecoded && (
            <span className='text-xs text-muted-foreground ml-1'>
              (Decoded)
            </span>
          )}
        </div>
        {isHexMessage && (
          <div className='flex items-center gap-2'>
            <span className='text-sm text-muted-foreground whitespace-nowrap'>
              Show decoded
            </span>
            <Switch checked={showDecoded} onCheckedChange={setShowDecoded} />
          </div>
        )}
      </div>
      <div className='text-sm text-muted-foreground break-all font-mono bg-muted p-2 rounded whitespace-pre-wrap'>
        {message}
      </div>
    </div>
  );
}

function SignMessageDialog({
  params,
}: CommandDialogProps<'chip0002_signMessage'>) {
  return (
    <div className='space-y-4 p-4'>
      <div className='space-y-2'>
        <div className='font-medium'>Public Key</div>
        <div className='text-sm text-muted-foreground break-all font-mono bg-muted p-2 rounded'>
          {params.publicKey}
        </div>
      </div>
      <MessageToSign message={params.message ?? ''} />
    </div>
  );
}

function SignMessageByAddressDialog({
  params,
}: CommandDialogProps<'chia_signMessageByAddress'>) {
  return (
    <div className='space-y-4 p-4'>
      <div className='space-y-2'>
        <div className='font-medium'>Address</div>
        <div className='text-sm text-muted-foreground break-all font-mono bg-muted p-2 rounded'>
          {params.address}
        </div>
      </div>
      <MessageToSign message={params.message ?? ''} />
    </div>
  );
}

function TakeOfferDialog({ params }: CommandDialogProps<'chia_takeOffer'>) {
  const [offer, setOffer] = useState<OfferSummary | null>(null);
  const { addError } = useErrors();

  useEffect(() => {
    commands
      .viewOffer({ offer: params.offer ?? '' })
      .then((data) => setOffer(data.offer))
      .catch(addError);
  }, [params, addError]);

  return offer ? (
    <OfferCard summary={offer} />
  ) : (
    <div className='p-4 text-center'>
      <Trans>Loading offer details...</Trans>
    </div>
  );
}

function CreateOfferDialog({ params }: CommandDialogProps<'chia_createOffer'>) {
  const walletState = useWalletState();
  // Check if any requested assets are revocable
  const hasRevocableAssets = params.requestAssets?.some(
    (asset) => asset.hiddenPuzzleHash,
  );

  return (
    <div className='space-y-4 p-4'>
      {hasRevocableAssets && (
        <div className='rounded-lg bg-amber-50 dark:bg-amber-950 p-4 border border-amber-200 dark:border-amber-800'>
          <div className='flex items-start gap-3'>
            <AlertTriangleIcon className='h-5 w-5 text-amber-500 mt-0.5' />
            <div>
              <h4 className='font-medium text-amber-800 dark:text-amber-200'>
                Warning: Revocable Assets
              </h4>
              <p className='text-sm text-amber-700 dark:text-amber-300 mt-1'>
                One or more assets being requested are revocable. These assets
                can be revoked by their issuer at any time. Please verify the
                validity of these assets before proceeding.
              </p>
            </div>
          </div>
        </div>
      )}
      <div>
        <div className='font-medium mb-2'>Offering</div>
        <ul className='list-disc list-inside space-y-1'>
          {params.offerAssets?.map((asset) => (
            <li key={asset.assetId} className='text-sm'>
              {formatNumber({
                value: fromMojos(
                  asset.amount,
                  asset.assetId === '' ? walletState.sync.unit.precision : 3,
                ),
                minimumFractionDigits: 0,
                maximumFractionDigits:
                  asset.assetId === '' ? walletState.sync.unit.precision : 3,
              })}{' '}
              {asset.assetId || 'XCH'}
            </li>
          ))}
        </ul>
      </div>
      <div>
        <div className='font-medium mb-2'>Requesting</div>
        <ul className='list-disc list-inside space-y-1'>
          {params.requestAssets?.map((asset) => (
            <li key={asset.assetId} className='text-sm'>
              {formatNumber({
                value: fromMojos(
                  asset.amount,
                  asset.assetId === '' ? walletState.sync.unit.precision : 3,
                ),
                minimumFractionDigits: 0,
                maximumFractionDigits:
                  asset.assetId === '' ? walletState.sync.unit.precision : 3,
              })}{' '}
              {asset.assetId || 'XCH'}
            </li>
          ))}
        </ul>
      </div>
      <div>
        <div className='font-medium'>Fee</div>
        <div className='text-sm text-muted-foreground'>
          {formatNumber({
            value: fromMojos(params.fee || 0, walletState.sync.unit.precision),
            minimumFractionDigits: 0,
            maximumFractionDigits: walletState.sync.unit.precision,
          })}{' '}
          {walletState.sync.unit.ticker}
        </div>
      </div>
    </div>
  );
}

function CancelOfferDialog({ params }: CommandDialogProps<'chia_cancelOffer'>) {
  const walletState = useWalletState();
  const [record, setRecord] = useState<OfferRecord | null>(null);
  const { addError } = useErrors();

  useEffect(() => {
    commands
      .getOffer({ offer_id: params.id ?? '' })
      .then((data) => setRecord(data.offer))
      .catch(addError);
  }, [params, addError]);

  return (
    <div className='space-y-2 p-4'>
      <div className='font-medium'>Offer ID</div>
      <div className='text-sm text-muted-foreground'>{params.id}</div>

      <div className='font-medium'>Fee</div>
      <div className='text-sm text-muted-foreground'>
        {formatNumber({
          value: fromMojos(params.fee || 0, walletState.sync.unit.precision),
          minimumFractionDigits: 0,
          maximumFractionDigits: walletState.sync.unit.precision,
        })}{' '}
        {walletState.sync.unit.ticker}
      </div>

      {record && (
        <div className='border rounded-md'>
          <OfferSummaryCard record={record} content={null} />
        </div>
      )}
    </div>
  );
}

function SendDialog({ params }: CommandDialogProps<'chia_send'>) {
  const walletState = useWalletState();

  return (
    <div className='space-y-2 p-4'>
      <div>
        <div className='font-medium'>Address</div>
        <div className='text-sm truncate text-muted-foreground'>
          {params.address}
        </div>
      </div>
      <div>
        <div className='font-medium'>Amount</div>
        <div className='text-sm text-muted-foreground'>
          {formatNumber({
            value: fromMojos(
              params.amount ?? 0,
              params.assetId ? 3 : walletState.sync.unit.precision,
            ),
            minimumFractionDigits: 0,
            maximumFractionDigits: params.assetId
              ? 3
              : walletState.sync.unit.precision,
          })}{' '}
          {params.assetId ? 'CAT' : walletState.sync.unit.ticker}
        </div>
      </div>
      <div>
        <div className='font-medium'>Fee</div>
        <div className='text-sm text-muted-foreground'>
          {formatNumber({
            value: fromMojos(params.fee || 0, walletState.sync.unit.precision),
            minimumFractionDigits: 0,
            maximumFractionDigits: walletState.sync.unit.precision,
          })}{' '}
          {walletState.sync.unit.ticker}
        </div>
      </div>
      {params.assetId && (
        <div>
          <div className='font-medium'>Asset Id</div>
          <div className='text-sm text-muted-foreground'>{params.assetId}</div>
        </div>
      )}
    </div>
  );
}

function DefaultCommandDialog({ params }: { params: unknown }) {
  return (
    <div className='p-4'>
      <div className='text-sm text-muted-foreground'>Command parameters:</div>
      <pre className='mt-2 rounded bg-muted p-4 overflow-auto'>
        <code className='text-xs'>{JSON.stringify(params, null, 2)}</code>
      </pre>
    </div>
  );
}

export const COMMAND_COMPONENTS: {
  [K in WalletConnectCommand]?: (props: CommandDialogProps<K>) => JSX.Element;
} = {
  chip0002_connect: ConnectDialog,
  chip0002_signCoinSpends: SignCoinSpendsDialog,
  chip0002_signMessage: SignMessageDialog,
  chia_takeOffer: TakeOfferDialog,
  chia_createOffer: CreateOfferDialog,
  chia_cancelOffer: CancelOfferDialog,
  chia_send: SendDialog,
  chia_signMessageByAddress: SignMessageByAddressDialog,
};

/** A site's own icon, falling back to its initial when there isn't one. */
function SiteMark({ origin, host }: { origin: string; host: string }) {
  const [failed, setFailed] = useState(false);

  if (failed || !origin) {
    return (
      <div className='h-16 w-16 rounded-full bg-muted flex items-center justify-center text-2xl font-medium uppercase'>
        {host.slice(0, 1)}
      </div>
    );
  }

  return (
    <img
      src={`${origin}/favicon.ico`}
      alt=''
      className='h-16 w-16 rounded-full object-contain'
      onError={() => setFailed(true)}
    />
  );
}

/**
 * Shown when a website asks to connect. The address the request came from is
 * what tells a real site from a lookalike, so it leads, and the account the
 * site would reach is named rather than assumed.
 */
function ConnectDialog({ peerName }: { peerName?: string | null }) {
  const { wallet } = useWallet();
  const walletState = useWalletState();
  const address = walletState.sync.receive_address;

  const origin = peerName ?? '';
  const separator = origin.indexOf('://');
  const scheme = separator === -1 ? '' : origin.slice(0, separator + 3);
  const host = separator === -1 ? origin : origin.slice(separator + 3);
  const insecure = scheme === 'http://';

  return (
    <div className='flex flex-col gap-6'>
      <div className='flex flex-col items-center gap-3 text-center'>
        <SiteMark origin={origin} host={host} />
        <div>
          <div className='text-lg font-medium font-mono break-all'>{host}</div>
          <div className='text-sm text-muted-foreground font-mono break-all'>
            {origin}
          </div>
          {insecure && (
            <div className='text-sm text-destructive mt-1'>
              <Trans>Not a secure connection</Trans>
            </div>
          )}
        </div>
      </div>

      <div className='border-t pt-4'>
        <div className='text-sm text-muted-foreground'>
          <Trans>Current account</Trans>
        </div>
        <div className='mt-2 flex items-center gap-3 rounded-md bg-muted px-3 py-2'>
          <div className='h-8 w-8 rounded-full bg-background flex items-center justify-center text-base'>
            {wallet?.emoji ?? '👤'}
          </div>
          <div className='min-w-0'>
            <div className='font-medium truncate'>{wallet?.name}</div>
            <div className='text-sm text-muted-foreground font-mono'>
              {formatAddress(address, 8, 4)}
            </div>
          </div>
        </div>
      </div>

      <div className='border-t pt-4'>
        <div className='text-sm text-muted-foreground'>
          <Trans>This site would like to</Trans>
        </div>
        <ul className='mt-3 flex flex-col gap-3'>
          <li className='flex items-center gap-3'>
            <CheckCircle2Icon
              className='h-5 w-5 shrink-0 text-primary'
              aria-hidden='true'
            />
            <Trans>View your wallet balance and activity</Trans>
          </li>
          <li className='flex items-center gap-3'>
            <CheckCircle2Icon
              className='h-5 w-5 shrink-0 text-primary'
              aria-hidden='true'
            />
            <Trans>Request approval for transactions</Trans>
          </li>
        </ul>
      </div>
    </div>
  );
}

export const COMMAND_METADATA: Partial<
  Record<
    WalletConnectCommand,
    {
      title: string;
      description: string;
    }
  >
> = {
  chip0002_connect: {
    title: 'Connect Site',
    description: 'Would you like to let this site connect to your wallet?',
  },
  chip0002_signCoinSpends: {
    title: 'Sign Transaction',
    description: 'Review and approve the transaction details below',
  },
  chip0002_signMessage: {
    title: 'Sign Message',
    description: 'Sign a message with your private key',
  },
  chia_takeOffer: {
    title: 'Accept Offer',
    description: 'Review and accept the offer',
  },
  chia_createOffer: {
    title: 'Create Offer',
    description: 'Review and create the offer',
  },
  chia_cancelOffer: {
    title: 'Cancel Offer',
    description: 'Review and cancel the offer',
  },
  chia_signMessageByAddress: {
    title: 'Sign Message',
    description: "Sign a message with your address's private key",
  },
};

export function RequestDialog({
  method,
  params,
  peerName,
  approve,
  reject,
}: RequestDialogProps) {
  const [isApproving, setIsApproving] = useState(false);
  const { currentTheme } = useTheme();
  const commandInfo = walletConnectCommands[method];
  const metadata = COMMAND_METADATA[method] ?? {
    title: 'WalletConnect Request',
    description: `Would you like to authorize the "${method.split('_').slice(1).join(' ')}" request?`,
  };

  const CommandComponent = COMMAND_COMPONENTS[method] ?? DefaultCommandDialog;

  // The connect dialog leads with the origin, so the header would repeat it.
  const ownsPeerName = method === 'chip0002_connect';

  const parsedParams = useMemo(
    () => commandInfo.paramsType.parse(params),
    [params, commandInfo],
  );

  const style: React.CSSProperties = {
    backgroundImage: currentTheme?.backgroundImage
      ? `url(${currentTheme.backgroundImage})`
      : undefined,
    backgroundSize: currentTheme?.backgroundImage ? 'cover' : undefined,
    backgroundPosition: currentTheme?.backgroundImage ? 'center' : undefined,
    backgroundRepeat: currentTheme?.backgroundImage ? 'no-repeat' : undefined,
  };

  if (currentTheme?.backgroundImage) {
    style.backgroundColor =
      currentTheme?.inherits === 'dark'
        ? 'rgba(0, 0, 0, 0.5)'
        : 'rgba(255, 255, 255, 0.5)';
    style.backgroundBlendMode = 'overlay';
  }

  return (
    <Dialog open={true} onOpenChange={(open) => !open && reject()}>
      {/* In the extension the request is the only thing on screen, so it takes
          the whole popup instead of floating in it. */}
      <DialogContent
        className={
          __IS_EXTENSION__ ? 'border-0 flex flex-col gap-0' : 'max-w-2xl'
        }
        style={__IS_EXTENSION__ ? { ...style, ...fullPopup } : style}
      >
        <DialogHeader>
          {peerName && !ownsPeerName && (
            <div className='text-sm text-muted-foreground mb-4'>
              From {peerName}
            </div>
          )}
          <DialogTitle>{metadata.title}</DialogTitle>
          <DialogDescription>{metadata.description}</DialogDescription>
        </DialogHeader>

        <div
          className={
            __IS_EXTENSION__
              ? 'flex-1 min-h-0 overflow-y-auto py-4'
              : 'max-h-[60vh] overflow-y-auto mb-2'
          }
        >
          {CommandComponent && (
            <CommandComponent params={parsedParams ?? {}} peerName={peerName} />
          )}
        </div>

        <DialogFooter
          className={
            __IS_EXTENSION__ ? 'flex-row gap-3 sm:justify-stretch' : ''
          }
        >
          <DialogClose asChild>
            <Button
              variant='outline'
              size={__IS_EXTENSION__ ? 'lg' : 'default'}
              className={__IS_EXTENSION__ ? 'flex-1' : ''}
              onClick={() => reject()}
            >
              <Trans>Reject</Trans>
            </Button>
          </DialogClose>
          <LoadingButton
            loading={isApproving}
            loadingText={t`Approving`}
            size={__IS_EXTENSION__ ? 'lg' : 'default'}
            className={__IS_EXTENSION__ ? 'flex-1' : ''}
            onClick={async () => {
              setIsApproving(true);
              try {
                await approve();
              } finally {
                setIsApproving(false);
              }
            }}
          >
            <Trans>Approve</Trans>
          </LoadingButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
