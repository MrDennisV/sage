// Content Bridge - ISOLATED world content script
// Relays messages between the page (inject.ts in MAIN world)
// and the extension service worker (chrome.runtime)

// Listen for messages from the page (inject.ts)
window.addEventListener('message', async (event) => {
  if (event.source !== window) return;
  if (event.data?.type !== 'SAGE_DAPP_REQUEST') return;

  const { id, method, params } = event.data;

  try {
    const response = await chrome.runtime.sendMessage({
      type: 'DAPP_REQUEST',
      method,
      params,
    });

    if (response?.error) {
      window.postMessage(
        {
          type: 'SAGE_DAPP_RESPONSE',
          id,
          error: response.error.reason || String(response.error),
        },
        '*',
      );
    } else {
      window.postMessage(
        {
          type: 'SAGE_DAPP_RESPONSE',
          id,
          result: response?.data,
        },
        '*',
      );
    }
  } catch (error: any) {
    window.postMessage(
      {
        type: 'SAGE_DAPP_RESPONSE',
        id,
        error: error?.message || String(error),
      },
      '*',
    );
  }
});

// Forward sync events from service worker to the page
chrome.runtime.onMessage.addListener((message) => {
  if (message?.type === 'SAGE_EVENT') {
    window.postMessage(
      {
        type: 'SAGE_EVENT',
        eventName: message.eventName,
        data: message.data,
      },
      '*',
    );
  }
});

// Inject the MAIN world script
const script = document.createElement('script');
script.src = chrome.runtime.getURL('inject.js');
script.type = 'module';
(document.head || document.documentElement).appendChild(script);
script.onload = () => script.remove();
