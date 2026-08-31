import type { StreamConfig } from "@/types";

/// Returns an i18n key (under "errors.*") describing the issue, or null if OK.
export function validateStreamConfig(config: StreamConfig | null): string | null {
  if (!config) return "errors.noServer";
  if (!config.deviceId) return "errors.noDevice";
  if (!config.host.trim()) return "errors.missingHost";
  if (config.port <= 0 || config.port > 65535) return "errors.badPort";
  if (!config.mount.trim()) return "errors.missingMount";
  if (!config.mount.startsWith("/")) return "errors.mountSlash";
  if (!config.username.trim()) return "errors.missingUsername";
  if (config.transport === "webcast") {
    // AzuraCast splits a packed "user:pass" password when the username is
    // empty or the literal "source" — and it looks for a comma across the
    // whole string *before* it looks for a colon. A real password containing
    // either separator would be silently mangled into wrong credentials.
    if (config.username.trim() === "source" && /[,:]/.test(config.password)) {
      return "errors.webdjSourceSplit";
    }
  }
  return null;
}
