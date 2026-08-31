import {
  DEFAULT_CONFIG,
  TRANSPORT_DEFAULT_PORT,
  type Bitrate,
  type StreamConfig,
  type StreamFormat,
  type Transport,
} from "@/types";

export interface ParsedServerLink {
  name: string;
  config: StreamConfig;
}

const VALID_BITRATES: Bitrate[] = [64, 128, 192, 320];

/**
 * Parse a deep-link URL like:
 *   `aircast://add-server?name=Prod&host=example.com&port=8000&mount=/live.mp3&user=source&pass=secret&format=mp3&bitrate=128`
 *
 * Add `transport=webcast` for an AzuraCast Web DJ endpoint, in which case
 * `mount` is the WebDJ path (`/webdj/<station>/`) and `port` defaults to 443:
 *   `aircast://add-server?host=stream.radios.bzh&transport=webcast&mount=/webdj/my-station/&user=dj`
 *
 * Required: `host`. Everything else falls back to sensible defaults.
 * Returns `null` if the URL isn't a valid Aircast add-server link.
 */
export function parseServerLink(url: string, deviceId: string): ParsedServerLink | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }
  if (parsed.protocol !== "aircast:") return null;

  // Accept both `aircast://add-server?…` (URL host) and `aircast:add-server?…`
  // (URL path), since some launchers normalize differently.
  const action =
    parsed.host || parsed.pathname.replace(/^\/+/, "").split("/")[0] || "";
  if (action !== "add-server") return null;

  const params = parsed.searchParams;
  const host = params.get("host")?.trim();
  if (!host) return null;

  const transport: Transport =
    params.get("transport")?.toLowerCase() === "webcast" ? "webcast" : "icecast";

  const portRaw = params.get("port");
  const port = portRaw
    ? parseInt(portRaw, 10)
    : TRANSPORT_DEFAULT_PORT[transport];
  if (!Number.isFinite(port) || port <= 0 || port > 65535) return null;

  const mountRaw = params.get("mount") ?? DEFAULT_CONFIG.mount;
  const mount = mountRaw.startsWith("/") ? mountRaw : `/${mountRaw}`;

  const username = params.get("user")?.trim() || DEFAULT_CONFIG.username;
  const password = params.get("pass") ?? DEFAULT_CONFIG.password;

  const formatRaw = params.get("format")?.toLowerCase();
  const format: StreamFormat = formatRaw === "aac" ? "aac" : "mp3";

  const bitrateRaw = params.get("bitrate");
  const bitrateNum = bitrateRaw ? parseInt(bitrateRaw, 10) : DEFAULT_CONFIG.bitrate;
  const bitrate: Bitrate = VALID_BITRATES.includes(bitrateNum as Bitrate)
    ? (bitrateNum as Bitrate)
    : DEFAULT_CONFIG.bitrate;

  const rawName = params.get("name")?.trim();
  const name = rawName && rawName.length > 0 ? rawName : host;

  return {
    name,
    config: {
      deviceId,
      host,
      port,
      mount,
      username,
      password,
      format,
      bitrate,
      transport,
    },
  };
}

/** Pick a non-conflicting name by appending " (2)", " (3)" … */
export function uniquePresetName(base: string, existingNames: string[]): string {
  if (!existingNames.includes(base)) return base;
  let i = 2;
  while (existingNames.includes(`${base} (${i})`)) i++;
  return `${base} (${i})`;
}
