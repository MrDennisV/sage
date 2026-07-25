// The popup end of the browser extension's dApp bridge. A website's request
// that needs a decision is parked in the service worker, which opens this popup
// and hands the request over a port; the user then answers it with the very
// same confirmation dialog WalletConnect uses, and the answer travels back.
import { useBiometric } from '@/hooks/useBiometric';
import { useErrors } from '@/hooks/useErrors';
import { WalletConnectCommand } from '@/walletconnect/commands';
import { handleCommand } from '@/walletconnect/handler';
import { RequestDialog } from '@/walletconnect/RequestDialog';
import { ReactNode, useCallback, useEffect, useState } from 'react';

const UI_PORT = 'sage-dapp-ui';

interface DappRequest {
  id: string;
  method: WalletConnectCommand;
  params: unknown;
  origin: string;
}

export function DappRequestProvider({ children }: { children?: ReactNode }) {
  const { addError } = useErrors();
  const { promptIfEnabled } = useBiometric();
  const [request, setRequest] = useState<DappRequest | null>(null);

  useEffect(() => {
    if (!__IS_EXTENSION__) return;

    // Connecting is also how the service worker learns the popup is up: it
    // answers with whatever is pending, and treats a disconnect as the user
    // walking away.
    const port = chrome.runtime.connect({ name: UI_PORT });

    port.onMessage.addListener((message) => {
      if (message?.type === 'DAPP_PENDING') {
        setRequest(message.request ?? null);
      }

      // The wallet was opened to answer a request, so it goes away with it.
      if (message?.type === 'DAPP_CLOSE') {
        window.close();
      }
    });

    return () => port.disconnect();
  }, []);

  const respond = useCallback(
    (id: string, payload: { result?: unknown; error?: string }) => {
      chrome.runtime
        .sendMessage({ type: 'DAPP_RESULT', id, ...payload })
        .catch(() => {
          // The service worker restarted; the request is already rejected.
        });
    },
    [],
  );

  const approve = useCallback(async () => {
    if (!request) return;

    try {
      const result = await handleCommand(request.method, request.params, {
        promptIfEnabled,
      });

      respond(request.id, { result });
    } catch (error) {
      const reason =
        error instanceof Error
          ? error.message
          : typeof error === 'object' && error !== null && 'reason' in error
            ? String(error.reason)
            : 'Request failed';

      addError({ kind: 'walletconnect', reason });
      respond(request.id, { error: reason });
    } finally {
      setRequest(null);
    }
  }, [request, promptIfEnabled, respond, addError]);

  const reject = useCallback(() => {
    if (!request) return;

    respond(request.id, { error: 'The user rejected the request' });
    setRequest(null);
  }, [request, respond]);

  return (
    <>
      {children}
      {request && (
        <RequestDialog
          method={request.method}
          params={request.params}
          peerName={request.origin}
          approve={approve}
          reject={reject}
        />
      )}
    </>
  );
}
