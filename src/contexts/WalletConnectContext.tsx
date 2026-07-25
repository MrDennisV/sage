import { commands } from '@/bindings';
import { useWallet } from '@/contexts/WalletContext';
import { useBiometric } from '@/hooks/useBiometric';
import { useErrors } from '@/hooks/useErrors';
import {
  WalletConnectCommand,
  walletConnectCommands,
} from '@/walletconnect/commands';
import { handleCommand } from '@/walletconnect/handler';
import { RequestDialog } from '@/walletconnect/RequestDialog';
import { getCurrentWindow, UserAttentionType } from '@tauri-apps/api/window';
import { platform } from '@tauri-apps/plugin-os';
import SignClient from '@walletconnect/sign-client';
import { SessionTypes, SignClientTypes } from '@walletconnect/types';
import {
  createContext,
  ReactNode,
  useCallback,
  useEffect,
  useState,
} from 'react';

export interface WalletConnectContextType {
  sessions: SessionTypes.Struct[];
  pair: (uri: string) => Promise<void>;
  disconnect: (topic: string) => Promise<void>;
  connecting: boolean;
}

export const WalletConnectContext = createContext<
  WalletConnectContextType | undefined
>(undefined);

type SessionRequest = SignClientTypes.EventArguments['session_request'];

export function WalletConnectProvider({ children }: { children: ReactNode }) {
  const { wallet } = useWallet();
  const { addError } = useErrors();
  const { promptIfEnabled } = useBiometric();

  const [signClient, setSignClient] = useState<Awaited<
    ReturnType<typeof SignClient.init>
  > | null>(null);
  const [sessions, setSessions] = useState<SessionTypes.Struct[]>([]);
  const [pendingRequests, setPendingRequests] = useState<SessionRequest[]>([]);
  const [connecting, setConnecting] = useState(false);

  useEffect(() => {
    SignClient.init({
      projectId: '7a11dea2c7ab88dc4597d5d44eb79a18',
      relayUrl: 'wss://relay.walletconnect.org',
      metadata: {
        name: 'Sage Wallet',
        description: 'Sage Wallet',
        url: 'https://sagewallet.net',
        icons: [
          'https://github.com/xch-dev/sage/blob/main/src-tauri/icons/icon.png?raw=true',
        ],
      },
    }).then((client) => {
      setSignClient(client);
    });
  }, []);

  const handleAndRespond = useCallback(
    async (request: SessionRequest) => {
      if (!signClient) {
        console.error('Sign client not initialized');
        return;
      }

      try {
        const method = request.params.request
          .method as keyof typeof walletConnectCommands;
        const result = await handleCommand(
          method,
          request.params.request.params,
          { promptIfEnabled },
        );

        await signClient.respond({
          topic: request.topic,
          response: {
            id: request.id,
            jsonrpc: '2.0',
            result: result,
          },
        });
      } catch (error) {
        const errorMessage =
          error instanceof Error
            ? error.message
            : typeof error === 'object' && error !== null && 'reason' in error
              ? (error.reason as string)
              : 'Request failed';
        addError({ kind: 'walletconnect', reason: errorMessage });
        console.error('WalletConnect request failed:', error);

        await signClient.respond({
          topic: request.topic,
          response: {
            id: request.id,
            jsonrpc: '2.0',
            error: {
              code: 4001,
              message: errorMessage,
            },
          },
        });
      }
    },
    [signClient, addError, promptIfEnabled],
  );

  useEffect(() => {
    if (!signClient) return;

    setSessions(signClient.session.getAll());

    async function handleSessionProposal(
      proposal: SignClientTypes.EventArguments['session_proposal'],
    ) {
      if (!signClient) {
        console.error('Sign client not initialized');
        return;
      }

      try {
        const {
          params: { pairingTopic, requiredNamespaces, optionalNamespaces },
        } = proposal;

        if (!pairingTopic) {
          throw new Error('Pairing topic not found');
        }

        const requiredNamespace =
          requiredNamespaces.chia || optionalNamespaces.chia;
        if (!requiredNamespace) {
          throw new Error('Missing required chia namespace');
        }

        const { chains, methods, events } = requiredNamespace;
        const chain = chains?.find((item) =>
          ['chia:testnet', 'chia:mainnet'].includes(item),
        );
        if (!chain) {
          throw new Error('Chain not supported');
        }

        const network = await commands.getNetwork({});

        if (!wallet) {
          throw new Error('No active wallet');
        }

        const account = `chia:${network.kind}:${wallet.fingerprint}`;
        const availableMethods = methods;
        const availableEvents = events;

        const { acknowledged } = await signClient.approve({
          id: proposal.id,
          namespaces: {
            chia: {
              accounts: [account],
              methods: availableMethods,
              events: availableEvents,
            },
          },
        });

        await acknowledged();
        setSessions(signClient.session.getAll());
        setConnecting(false);
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : 'Failed to connect';
        addError({ kind: 'walletconnect', reason: errorMessage });
        console.error('WalletConnect session proposal failed:', error);
        setConnecting(false);

        await signClient.reject({
          id: proposal.id,
          reason: {
            code: 4001,
            message: errorMessage,
          },
        });
      }
    }

    async function handleSessionRequest(request: SessionRequest) {
      try {
        const method = request.params.request
          .method as keyof typeof walletConnectCommands;

        if (!walletConnectCommands[method]) {
          throw new Error(`Unsupported method: ${method}`);
        }

        try {
          walletConnectCommands[method].paramsType.parse(
            request.params.request.params,
          );
        } catch (error) {
          console.error('Invalid parameters for method:', method, error);
          throw new Error(
            error instanceof Error
              ? error.message
              : `Invalid parameters for ${method}`,
          );
        }

        if (walletConnectCommands[method].confirm) {
          setPendingRequests((p: SessionRequest[]) => [...p, request]);
          const os = platform();
          if (os === 'macos' || os === 'windows' || os === 'linux') {
            await getCurrentWindow().requestUserAttention(
              UserAttentionType.Critical,
            );
          }
        } else {
          await handleAndRespond(request);
        }
      } catch (error) {
        console.error('WalletConnect session request failed:', error);

        if (signClient) {
          await signClient.respond({
            topic: request.topic,
            response: {
              id: request.id,
              jsonrpc: '2.0',
              error: {
                code: 4001,
                message:
                  error instanceof Error ? error.message : 'Request failed',
              },
            },
          });
        }
      }
    }

    async function handleSessionDelete() {
      if (!signClient) throw new Error('Sign client not initialized');

      setSessions(signClient.session.getAll());
    }

    signClient.on('session_proposal', handleSessionProposal);
    signClient.on('session_request', handleSessionRequest);
    signClient.on('session_delete', handleSessionDelete);
    return () => {
      signClient.off('session_proposal', handleSessionProposal);
      signClient.off('session_request', handleSessionRequest);
      signClient.off('session_delete', handleSessionDelete);
    };
  }, [signClient, wallet, handleAndRespond, setPendingRequests, addError]);

  const pair = async (uri: string) => {
    if (!signClient) {
      console.error('Sign client not initialized');
      return;
    }

    try {
      setConnecting(true);
      await signClient.core.pairing.pair({ uri });
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : 'Failed to pair';
      addError({ kind: 'walletconnect', reason: errorMessage });
      console.error('WalletConnect pairing failed:', error);
      setConnecting(false);
    }
  };

  const disconnect = async (topic: string) => {
    if (!signClient) {
      console.error('Sign client not initialized');
      return;
    }

    try {
      await signClient.disconnect({
        topic,
        reason: { code: 4001, message: 'User disconnected' },
      });
      setSessions(signClient.session.getAll());
    } catch (error) {
      console.error('WalletConnect disconnect failed:', error);
    }
  };

  const approveRequest = async (request: SessionRequest) => {
    if (!pendingRequests.find((r) => r.id === request.id)) {
      return;
    }

    await handleAndRespond(request);
    setPendingRequests((p: SessionRequest[]) =>
      p.filter((r) => r.id !== request.id),
    );
  };

  const rejectRequest = async (request: SessionRequest) => {
    if (!signClient) throw new Error('Sign client not initialized');

    if (!pendingRequests.find((r) => r.id === request.id)) {
      return;
    }

    await signClient.respond({
      topic: request.topic,
      response: {
        id: request.id,
        jsonrpc: '2.0',
        result: null,
      },
    });
    setPendingRequests((p: SessionRequest[]) =>
      p.filter((r) => r.id !== request.id),
    );
  };

  const request = pendingRequests[0];

  return (
    <WalletConnectContext.Provider
      value={{ pair, sessions, disconnect, connecting }}
    >
      {children}
      {request && (
        <RequestDialog
          method={request.params.request.method as WalletConnectCommand}
          params={request.params.request.params}
          peerName={signClient?.session.get(request.topic)?.peer.metadata.name}
          approve={() => approveRequest(request)}
          reject={() => rejectRequest(request)}
        />
      )}
    </WalletConnectContext.Provider>
  );
}
