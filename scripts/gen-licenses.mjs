// Generates src/licenses.json: third-party open-source licenses for the
// Rust crates (cargo metadata) and npm packages (license-checker) the app uses.
// Run with: npm run licenses
import { execSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const SELF = "nfc-keyboard-emulator";

function rust() {
  const raw = execSync("cargo metadata --format-version 1", {
    cwd: join(root, "src-tauri"),
    maxBuffer: 128 * 1024 * 1024,
  }).toString();
  const meta = JSON.parse(raw);
  const map = new Map();
  for (const p of meta.packages) {
    if (p.name === SELF) continue;
    map.set(`${p.name}@${p.version}`, {
      name: p.name,
      version: p.version,
      license: p.license || (p.license_file ? "see license file" : "—"),
    });
  }
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
}

function npm() {
  const raw = execSync("npx --yes license-checker --production --json", {
    cwd: root,
    maxBuffer: 128 * 1024 * 1024,
  }).toString();
  const data = JSON.parse(raw);
  const out = [];
  for (const [key, value] of Object.entries(data)) {
    const at = key.lastIndexOf("@");
    const name = key.slice(0, at);
    if (name === SELF) continue;
    const lic = value.licenses;
    out.push({
      name,
      version: key.slice(at + 1),
      license: Array.isArray(lic) ? lic.join(" / ") : lic || "—",
    });
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

const result = { rust: rust(), npm: npm() };
writeFileSync(join(root, "src/licenses.json"), JSON.stringify(result));
console.log(`Wrote src/licenses.json — rust: ${result.rust.length}, npm: ${result.npm.length}`);
