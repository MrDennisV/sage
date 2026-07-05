import { PenLine } from 'lucide-react';
import type { RustBridgeApprovalRequest } from '@sage-system-app/sdk';
import { ApprovalDetailRow, ApprovalMetaPill } from './shared';

interface Props {
  approval: Extract<RustBridgeApprovalRequest, { kind: 'signCoinSpends' }>;
  appName: string;
  expanded: boolean;
}

export function SignCoinSpendsApprovalCard({ approval, appName }: Props) {
  const coinSpends = approval.coin_spends;
  const json = JSON.stringify(coinSpends, null, 2);

  return (
    <div className='space-y-3'>
      <div className='flex items-start gap-3'>
        <div className='rounded-xl border bg-background p-2 text-muted-foreground'>
          <PenLine className='h-4 w-4' />
        </div>

        <div className='min-w-0 flex-1'>
          <div className='flex flex-wrap items-center gap-2'>
            <div className='text-sm font-medium'>Sign coin spends</div>
            <ApprovalMetaPill>Wallet</ApprovalMetaPill>
          </div>

          <div className='mt-1 text-xs text-muted-foreground'>
            {appName} wants your wallet to sign coin spends it constructed. The
            wallet only signs — it does not broadcast them.
          </div>
        </div>
      </div>

      <div className='space-y-2 rounded-xl border bg-background/70 p-3'>
        <ApprovalDetailRow label='Coin spends' value={String(coinSpends.length)} />
      </div>

      <div className='rounded-xl border bg-background/70 p-3'>
        <div className='mb-2 text-xs font-medium text-muted-foreground'>
          Coin spends
        </div>
        <pre className='max-h-64 overflow-auto whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed'>
          {json}
        </pre>
      </div>
    </div>
  );
}
