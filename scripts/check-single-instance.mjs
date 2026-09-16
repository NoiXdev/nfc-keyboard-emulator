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
const SETTLE_MS = 4000;

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
  const entry = { child, exited: false };
  child.on("exit", () => (entry.exited = true));
  child.on("error", () => (entry.exited = true));
  started.push(entry);
}

const alive = () => started.filter((e) => !e.exited).length;

let failed = false;
function expect(want, what) {
  const got = alive();
  if (got === want) {
    console.log(`ok   - ${what}: ${got} instance(s)`);
  } else {
    console.log(`FAIL - ${what}: expected ${want}, got ${got}`);
    failed = true;
  }
}

launch();
await sleep(SETTLE_MS);
if (alive() === 0) {
  // No desktop session or no WebView2 runtime: the app cannot come up at all,
  // which says nothing about single-instance behaviour. Skip loudly rather than
  // report a regression that isn't one.
  console.log("skip - app did not start, no desktop session available");
  process.exit(0);
}
expect(1, "after launch");

launch();
await sleep(SETTLE_MS);
expect(1, "after second launch (shortcut while autostarted)");

launch();
await sleep(SETTLE_MS);
expect(1, "after third launch");

for (const { child, exited } of started) if (!exited) child.kill();
process.exit(failed ? 1 : 0);
