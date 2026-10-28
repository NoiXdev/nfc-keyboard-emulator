# Example server

A minimal server for trying out nfc-keyboard-emulator's server integration. It is
reference code to read and copy from, not something to run in production: the
browser page has no authentication and all state is kept in memory.

It needs Node.js 22 or newer.

```sh
npm install
TOKEN=dev node server.mjs   # PORT defaults to 8787
```

Open <http://localhost:8787/> for the demo page, then enter these in the app's
Server view (use the same token everywhere):

| Mode    | URL                              | Token |
| ------- | -------------------------------- | ----- |
| HTTP    | `http://localhost:8787/scans`    | `dev` |
| Stream  | `ws://localhost:8787/scanner`    | `dev` |
| Request | `ws://localhost:8787/scanner`    | `dev` |

- **HTTP / Stream:** every scan shows up in the page's live list.
- **Request:** click **Scan** in the page, then hold a card to the reader. The UID
  appears as the result of that request and is not typed.

`server.mjs` has comments on each protocol rule (token check on the upgrade, the
`hello` message, `scan`, `scan_request`, `scan_response` and `scan_request_ended`).
The full protocol is described in the
[server integration guide](https://docs.noix.dev/nfc-keyboard-emulator/guide/server-integration/).
