import { getVersion } from "@tauri-apps/api/app";

const REPO = "NoiXdev/nfc-keyboard-emulator";
const RELEASES_URL = `https://github.com/${REPO}/releases/latest`;

export interface UpdateInfo {
  current: string;
  latest: string | null;
  available: boolean;
  url: string;
  ok: boolean;
}

function compare(a: string, b: string): number {
  const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d > 0 ? 1 : -1;
  }
  return 0;
}

// Checks the latest GitHub release tag against the running app version.
export async function checkForUpdate(): Promise<UpdateInfo> {
  const current = await getVersion().catch(() => "");
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) throw new Error(String(res.status));
    const data = await res.json();
    const latest = String(data.tag_name ?? "").replace(/^v/, "");
    const url = typeof data.html_url === "string" ? data.html_url : RELEASES_URL;
    const available = Boolean(latest) && Boolean(current) && compare(latest, current) > 0;
    return { current, latest: latest || null, available, url, ok: true };
  } catch {
    return { current, latest: null, available: false, url: RELEASES_URL, ok: false };
  }
}
