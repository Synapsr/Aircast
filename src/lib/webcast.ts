import { TRANSPORT_DEFAULT_PORT, type StreamConfig, type Transport } from "@/types";

/**
 * Hosts for which plaintext `ws://` is allowed. Anything that resolves off
 * this machine must use TLS, because the credentials travel inside the first
 * WebSocket frame rather than in an HTTP header.
 *
 * Mirrors `is_loopback_host` in `src-tauri/src/presets/mod.rs` — keep the two
 * in sync, since the Rust side is what actually opens the socket.
 */
function isLoopbackHost(host: string): boolean {
  const h = host.trim().replace(/^\[/, "").replace(/\]$/, "");
  return (
    h.toLowerCase() === "localhost" ||
    h === "::1" ||
    h.startsWith("127.") ||
    h.toLowerCase().endsWith(".local")
  );
}

/** Mount with a guaranteed leading slash. */
export function normalizedMount(mount: string): string {
  return mount.startsWith("/") ? mount : `/${mount}`;
}

/**
 * The endpoint the webcast transport will actually connect to.
 *
 * Shown in Setup so the user can see exactly what Aircast will do with the
 * host / port / path fields — the WebSocket URL is not obvious from them.
 */
export function webcastUrl(
  config: Pick<StreamConfig, "host" | "port" | "mount">,
): string {
  const host = config.host.trim();
  const scheme = isLoopbackHost(host) ? "ws" : "wss";
  const defaultPort = scheme === "wss" ? 443 : 80;
  const authority =
    config.port === defaultPort ? host : `${host}:${config.port}`;
  return `${scheme}://${authority}${normalizedMount(config.mount)}`;
}

/**
 * The port to use after switching transport, unless the current one already
 * makes sense for the destination.
 *
 * The two transports live in completely different ranges, and leaving the old
 * value behind produces a puzzling timeout rather than an obvious mistake.
 * This matters more than it looks: an AzuraCast source port is 8005, 8015,
 * 8025… — never the 8000 default — so a rule that only moved *default* ports
 * would never fire for a real preset, and every Web DJ switch would silently
 * dial wss://host:8015/ and fail.
 */
export function portForTransport(currentPort: number, to: Transport): number {
  const TLS_PORTS = [443, 8443, 80];
  if (to === "webcast") {
    return TLS_PORTS.includes(currentPort)
      ? currentPort
      : TRANSPORT_DEFAULT_PORT.webcast;
  }
  return currentPort === 443 || currentPort === 8443
    ? TRANSPORT_DEFAULT_PORT.icecast
    : currentPort;
}
