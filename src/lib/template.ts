import type { AuthConfig, PayloadConfig } from "./tauri";

export const MAX_TEMPLATE_BYTES = 8192;

/** The names a `{{placeholder}}` may refer to; same order as the backend. */
export const PLACEHOLDERS = [
  "uid",
  "uidFormatted",
  "uidLength",
  "scanId",
  "device",
  "reader",
  "timestamp",
] as const;

export interface ScanPayload {
  scanId: string;
  device: string;
  reader: string;
  uid: string;
  uidFormatted: string;
  uidLength: number;
  timestamp: string;
}

export type TemplateError =
  | { kind: "not_json" }
  | { kind: "unknown_placeholder"; name: string }
  | { kind: "too_large" };

export type RenderResult = { ok: true; value: unknown } | { ok: false; error: TemplateError };

export const defaultAuth = (): AuthConfig => ({
  mode: "bearer",
  headerName: "X-API-Key",
  username: "",
  queryParam: "token",
});

export const defaultPayload = (): PayloadConfig => ({ mode: "standard", template: "" });

/** The scan the "send test" action sends: fixed values, so no card is needed. */
export const examplePayload = (device: string, now: Date = new Date()): ScanPayload => ({
  scanId: "test",
  device,
  reader: "Example Reader",
  uid: "04A1B2C3D4E5F6",
  uidFormatted: "04A1B2C3D4E5F6",
  uidLength: 7,
  timestamp: now.toISOString(),
});

/** The standard payload fields as a template, the starting point for a custom one. */
export const defaultTemplate = (): string =>
  JSON.stringify(
    {
      scanId: "{{scanId}}",
      device: "{{device}}",
      reader: "{{reader}}",
      uid: "{{uid}}",
      uidFormatted: "{{uidFormatted}}",
      uidLength: "{{uidLength}}",
      timestamp: "{{timestamp}}",
    },
    null,
    2,
  );

type Segment = { text: string } | { placeholder: string };

// Rust's `is_ascii_whitespace`; JS `\s` would also accept `\v`, NBSP and other Unicode spaces.
const isAsciiWhitespace = (c: string): boolean =>
  c === " " || c === "\t" || c === "\n" || c === "\f" || c === "\r";

const isIdentifier = (name: string): boolean => /^[A-Za-z_][A-Za-z0-9_]*$/.test(name);

function trimAscii(s: string): string {
  let start = 0;
  let end = s.length;
  while (start < end && isAsciiWhitespace(s[start])) start++;
  while (end > start && isAsciiWhitespace(s[end - 1])) end--;
  return s.slice(start, end);
}

/**
 * Splits a string at `{{ name }}` placeholders, like the backend: anything else, such as
 * `{{}}`, `{{ a b }}` or an unclosed `{{uid`, stays literal text. After a miss the scan
 * continues from the second brace, so `{{{uid}}}` still finds `{{uid}}`.
 */
function segments(s: string): Segment[] {
  const out: Segment[] = [];
  let rest = s;
  for (;;) {
    const open = rest.indexOf("{{");
    if (open < 0) break;
    const after = rest.slice(open + 2);
    const close = after.indexOf("}}");
    if (close < 0) break;
    const name = trimAscii(after.slice(0, close));
    if (isIdentifier(name)) {
      if (open > 0) out.push({ text: rest.slice(0, open) });
      out.push({ placeholder: name });
      rest = after.slice(close + 2);
    } else {
      out.push({ text: rest.slice(0, open + 1) });
      rest = rest.slice(open + 1);
    }
  }
  if (rest.length > 0) out.push({ text: rest });
  return out;
}

const isPlaceholder = (name: string): name is (typeof PLACEHOLDERS)[number] =>
  (PLACEHOLDERS as readonly string[]).includes(name);

/** The first unknown name in document order, depth-first. */
function findUnknown(value: unknown): string | null {
  if (typeof value === "string") {
    for (const seg of segments(value)) {
      if ("placeholder" in seg && !isPlaceholder(seg.placeholder)) return seg.placeholder;
    }
    return null;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = findUnknown(item);
      if (found !== null) return found;
    }
    return null;
  }
  if (value !== null && typeof value === "object") {
    for (const item of Object.values(value)) {
      const found = findUnknown(item);
      if (found !== null) return found;
    }
  }
  return null;
}

function fill(value: unknown, payload: ScanPayload): unknown {
  if (typeof value === "string") {
    const parts = segments(value);
    if (parts.length === 1 && "placeholder" in parts[0]) {
      return payload[parts[0].placeholder as keyof ScanPayload];
    }
    return parts
      .map((p) => ("text" in p ? p.text : String(payload[p.placeholder as keyof ScanPayload])))
      .join("");
  }
  if (Array.isArray(value)) return value.map((v) => fill(v, payload));
  if (value !== null && typeof value === "object") {
    // `fromEntries` defines own properties, so a `__proto__` key stays a plain key.
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, fill(v, payload)]));
  }
  return value;
}

/**
 * Substitutes the scan into every string value of the template; keys are never touched. A
 * string that is exactly one placeholder takes the placeholder's own type, so `"{{uidLength}}"`
 * becomes a number. Checks size (UTF-8 bytes), JSON syntax, then placeholder names.
 *
 * This is a preview, not the wire format: the backend serialises the result itself, so the
 * sent payload may differ in detail. JS orders integer-like keys (`"2"`, `"10"`) before the
 * others, numbers lose their notation (`1.0` becomes `1`, `1e2` becomes `100`), and
 * `JSON.parse` accepts a few inputs the backend's parser rejects (a lone surrogate escape, an
 * out-of-range number). The backend decides whether a template is valid.
 */
export function renderTemplate(template: string, payload: ScanPayload): RenderResult {
  if (new TextEncoder().encode(template).length > MAX_TEMPLATE_BYTES) {
    return { ok: false, error: { kind: "too_large" } };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(template);
  } catch {
    return { ok: false, error: { kind: "not_json" } };
  }
  const unknown = findUnknown(parsed);
  if (unknown !== null) return { ok: false, error: { kind: "unknown_placeholder", name: unknown } };
  return { ok: true, value: fill(parsed, payload) };
}
