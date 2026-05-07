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
  return null;
}
