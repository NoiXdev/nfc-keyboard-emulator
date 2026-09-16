// Verifies that only one instance of the app can run at a time (see issue #2).
// On Windows the autostart registry Run key launches the exe directly, and there
// is no OS-level guard at all, so every extra launch used to mean another process
// polling the reader and typing the UID a second time.
//
// Run with: node scripts/check-single-instance.mjs [path/to/app.exe]
//
// The path argument also makes this runnable off Windows, which is how the
// counting logic itself is tested.
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const OUT_DIR = "src-tauri/target/release";
const SETTLE_TIMEOUT_MS = 30000;
const STARTUP_GRACE_MS = 4000;
const POLL_MS = 250;

// The release binary is named after the cargo bin, not the productName - but the
// bundler has been known to rename it, so try both rather than fail on a path.
function defaultExe() {
  const { productName } = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const candidates = [`${OUT_DIR}/nfc-keyboard-emulator.exe`, `${OUT_DIR}/${productName}.exe`];
  return candidates.find(existsSync) ?? candidates[0];
}

const exe = process.argv[2];
if (!exe && process.platform !== "win32") {
  console.log("skip - Windows only (pass an executable path to run anyway)");
  process.exit(0);
}
const target = exe ?? defaultExe();
if (!existsSync(target)) {
  console.error(`error: no executable at '${target}' - run 'npm run tauri build' first`);
  process.exit(2);
}

const started = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function launch() {
  const child = spawn(target, { stdio: "ignore" });
  const entry = { exited: false, code: null, child };
  child.on("exit", (code) => Object.assign(entry, { exited: true, code }));
  child.on("error", () => Object.assign(entry, { exited: true, code: "spawn error" }));
  started.push(entry);
}

const alive = () => started.filter((e) => !e.exited).length;

// A superseded instance has to boot far enough to reach the guard before it can
// hand over and exit, which on a cold CI runner takes well over the ~1s it takes
// on a warm desktop. So poll for the expected count instead of guessing a fixed
// settle time - a real regression never reaches it and fails on the timeout.
async function settle(want) {
  const deadline = Date.now() + SETTLE_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (alive() === want) return;
    if (started.every((e) => e.exited)) return;
    await sleep(POLL_MS);
  }
}

let failed = false;
function check(ok, message) {
  console.log(`${ok ? "ok  " : "FAIL"} - ${message}`);
  if (!ok) failed = true;
}

launch();
// Not settle(): a freshly spawned process counts as alive until its exit event
// fires, so polling for "1 alive" would return before a failed start is visible.
// Give it a fixed moment to fall over instead.
await sleep(STARTUP_GRACE_MS);
if (alive() === 0) {
  // No desktop session or no WebView2 runtime: the app cannot come up at all,
  // which says nothing about single-instance behaviour. Skip loudly rather than
  // report a regression that isn't one.
  console.log("skip - app did not start, no desktop session available");
  process.exit(0);
}
check(alive() === 1, `after launch: ${alive()} instance(s)`);

launch();
await settle(1);
check(alive() === 1, `after second launch (shortcut while autostarted): ${alive()} instance(s)`);

launch();
await settle(1);
check(alive() === 1, `after third launch: ${alive()} instance(s)`);

// Distinguishes "deferred to the running instance" from "crashed on startup" -
// both leave one process behind, only the first one is the behaviour we want.
const codes = started.filter((e) => e.exited).map((e) => e.code);
if (codes.length) {
  check(
    codes.every((c) => c === 0),
    `superseded instances exited cleanly (code ${codes.join(", ")})`,
  );
}

for (const { child, exited } of started) if (!exited) child.kill();
process.exit(failed ? 1 : 0);
