// Example server for nfc-keyboard-emulator's optional server integration.
// Reference code, not production code: the browser socket has no auth and
// state lives in memory. Run: TOKEN=dev node server.mjs  (PORT defaults to 8787)
//
//   POST /scans   HTTP push target (bearer token required)
//   WS   /scanner the app's WebSocket (bearer token checked on the upgrade)
//   WS   /browser demo web page socket (no auth, demo only)
//   GET  /        the demo page
import { createServer } from "node:http";
import { randomUUID, timingSafeEqual } from "node:crypto";
import { readFile } from "node:fs/promises";
import { WebSocketServer } from "ws";

const PORT = Number(process.env.PORT ?? 8787);
const TOKEN = process.env.TOKEN ?? "";
if (!TOKEN) {
  console.error("Set TOKEN, e.g. TOKEN=dev node server.mjs");
  process.exit(1);
}

// The app sends the token as `Authorization: Bearer <token>`, both on HTTP
// pushes and on the WebSocket upgrade request. Compare in constant time;
// timingSafeEqual needs equal-length buffers, so lengths are compared first.
function authorized(req) {
  const given = Buffer.from((req.headers.authorization ?? "").replace(/^Bearer /i, ""));
  const expected = Buffer.from(TOKEN);
  return given.length === expected.length && timingSafeEqual(given, expected);
}

const browsers = new Set(); // sockets on /browser
let app = null; // the app's socket on /scanner (one app at a time in this demo)
const pending = new Map(); // requestId -> the browser socket that asked

const send = (ws, msg) => ws.readyState === ws.OPEN && ws.send(JSON.stringify(msg));
const broadcast = (msg) => browsers.forEach((b) => send(b, msg));

// A scan from the app, over HTTP or the WebSocket stream, has the same payload:
// { type: "scan", protocolVersion, scanId, device, reader, uid, uidFormatted,
//   uidLength, timestamp }. `uid` is canonical (uppercase hex, no separators).
// Servers must ignore unknown fields, since new ones may be added in v1.
function onScan(scan, via) {
  console.log(`scan via ${via}: ${scan.uid} from "${scan.device}"`);
  broadcast({ ...scan, via });
}

const server = createServer(async (req, res) => {
  const { pathname } = new URL(req.url, "http://localhost");

  if (req.method === "GET" && pathname === "/") {
    res.setHeader("content-type", "text/html; charset=utf-8");
    return res.end(await readFile(new URL("./index.html", import.meta.url)));
  }

  if (req.method === "POST" && pathname === "/scans") {
    // 401/403 shows up in the app as "token rejected"; any 2xx is success and
    // the response body is ignored. The app sends `test` (the "Send test"
    // button) and `scan` messages here.
    if (!authorized(req)) {
      res.writeHead(401).end();
      return;
    }
    let body = "";
    for await (const chunk of req) body += chunk;
    try {
      const msg = JSON.parse(body);
      if (msg.type === "scan") onScan(msg, "http");
      else console.log(`http ${msg.type} from "${msg.device}"`);
      res.writeHead(204).end();
    } catch {
      res.writeHead(400).end();
    }
    return;
  }

  res.writeHead(404).end();
});

const scanner = new WebSocketServer({ noServer: true });
const browser = new WebSocketServer({ noServer: true });

// Auth happens on the upgrade request, before any WebSocket exists. Answering
// 401 here makes the app show "token rejected" and stop reconnecting.
server.on("upgrade", (req, socket, head) => {
  const { pathname } = new URL(req.url, "http://localhost");
  if (pathname === "/scanner") {
    if (!authorized(req)) {
      socket.end("HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\n");
      return;
    }
    scanner.handleUpgrade(req, socket, head, (ws) => onAppConnected(ws));
  } else if (pathname === "/browser") {
    browser.handleUpgrade(req, socket, head, (ws) => onBrowserConnected(ws));
  } else {
    socket.destroy();
  }
});

function onAppConnected(ws) {
  app?.close(); // a newer connection replaces the old one
  app = ws;
  ws.on("close", () => {
    if (app === ws) app = null;
  });
  ws.on("message", (data) => {
    let msg;
    try {
      msg = JSON.parse(data.toString());
    } catch {
      return; // ignore anything that is not JSON
    }
    switch (msg.type) {
      case "hello": // first message after connecting: who, which version, which modes
        console.log(`app connected: "${msg.device}" v${msg.appVersion}`, msg.modes);
        break;
      case "scan": // WS stream mode: every scan
        onScan(msg, "ws");
        break;
      case "scan_response": // the scan that answered a scan_request
        // Same fields as a scan plus requestId. The requester gets the response
        // itself; every page also sees it as a scan in the live list.
        onScan({ ...msg, type: "scan" }, "ws");
        forwardToRequester(msg);
        break;
      case "scan_request_ended": // no scan came: timeout, cancelled, superseded or disabled
        console.log(`request ${msg.requestId} ended: ${msg.reason}`);
        forwardToRequester(msg);
        break;
      default:
        break; // unknown types are ignored
    }
  });
}

function forwardToRequester(msg) {
  const requester = pending.get(msg.requestId);
  pending.delete(msg.requestId);
  if (requester) send(requester, msg);
}

function onBrowserConnected(ws) {
  browsers.add(ws);
  ws.on("close", () => {
    browsers.delete(ws);
    // Disarm the app for requests whose page went away.
    for (const [requestId, owner] of pending) {
      if (owner !== ws) continue;
      pending.delete(requestId);
      if (app) send(app, { type: "scan_request_cancel", requestId });
    }
  });
  ws.on("message", (data) => {
    let msg;
    try {
      msg = JSON.parse(data.toString());
    } catch {
      return;
    }
    if (msg.type !== "request_scan") return;
    if (!app) return send(ws, { type: "scan_request_ended", reason: "app_not_connected" });
    // Ask the app for the next scan. timeoutMs is optional (default 30000,
    // capped at 300000). Only one request is pending in the app at a time: a
    // new one replaces the old one, which then ends as "superseded".
    const requestId = randomUUID();
    pending.set(requestId, ws);
    send(app, { type: "scan_request", requestId, timeoutMs: 30000 });
  });
}

server.listen(PORT, () => console.log(`listening on http://localhost:${PORT}`));
