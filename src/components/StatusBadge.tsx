import { useEffect, useState } from "react";
import { useT } from "@/i18n/context";
import type { StreamStatus } from "@/types";

interface Props {
  status: StreamStatus;
}

export function StatusBadge({ status }: Props) {
  const { t } = useT();
  const [reconnectCountdown, setReconnectCountdown] = useState<number | null>(null);

  useEffect(() => {
    if (status.kind !== "reconnecting") {
      setReconnectCountdown(null);
      return;
    }
    const target = Date.now() + status.nextAttemptInMs;
    setReconnectCountdown(Math.max(0, Math.ceil(status.nextAttemptInMs / 1000)));
    const id = setInterval(() => {
      const remaining = Math.max(0, Math.ceil((target - Date.now()) / 1000));
      setReconnectCountdown(remaining);
    }, 250);
    return () => clearInterval(id);
  }, [status]);

  const { dotColor, label, pulse, textColor } = (() => {
    switch (status.kind) {
      case "idle":
        return { dotColor: "bg-zinc-600", label: t("status.ready"), pulse: false, textColor: "text-zinc-400" };
      case "connecting":
        return { dotColor: "bg-amber-400", label: t("status.connecting"), pulse: true, textColor: "text-amber-300" };
      case "live":
        return { dotColor: "bg-rose-500", label: t("status.live"), pulse: true, textColor: "text-rose-300 font-medium" };
      case "reconnecting":
        return {
          dotColor: "bg-amber-400",
          label:
            reconnectCountdown !== null
              ? t("status.reconnectingIn", { seconds: reconnectCountdown })
              : t("status.reconnecting"),
          pulse: true,
          textColor: "text-amber-300",
        };
      case "error":
        return { dotColor: "bg-red-500", label: status.message, pulse: false, textColor: "text-red-400" };
    }
  })();

  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="relative flex h-2.5 w-2.5">
        {pulse && (
          <span
            className={`absolute inline-flex h-full w-full animate-ping rounded-full ${dotColor} opacity-60`}
          />
        )}
        <span className={`relative inline-flex h-2.5 w-2.5 rounded-full ${dotColor}`} />
      </span>
      <span className={textColor}>{label}</span>
    </div>
  );
}
