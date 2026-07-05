import { PenLine } from 'lucide-react';
import type { RustBridgeApprovalRequest } from '@sage-system-app/sdk';
import { ApprovalMetaPill } from './shared';

interface Props {
  approval: Extract<RustBridgeApprovalRequest, { kind: 'signMessage' }>;
  appName: string;
  expanded: boolean;
}

export function SignMessageApprovalCard({ approval, appName }: Props) {
  return (
    <div className='space-y-3'>
      <div className='flex items-start gap-3'>
        <div className='rounded-xl border bg-background p-2 text-muted-foreground'>
          <PenLine className='h-4 w-4' />
        </div>

        <div className='min-w-0 flex-1'>
          <div className='flex flex-wrap items-center gap-2'>
            <div className='text-sm font-medium'>Sign message</div>
            <ApprovalMetaPill>Wallet</ApprovalMetaPill>
          </div>

          <div className='mt-1 text-xs text-muted-foreground'>
            {appName} wants your wallet to sign a message with your key.
          </div>
        </div>
      </div>

      <div className='rounded-xl border bg-background/70 p-3'>
        <div className='mb-2 text-xs font-medium text-muted-foreground'>
          Message
        </div>
        <div className='max-h-40 overflow-auto whitespace-pre-wrap break-all font-mono text-xs'>
          {approval.message}
        </div>
      </div>
    </div>
  );
}
