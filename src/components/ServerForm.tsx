import { Lock, ShieldAlert } from "lucide-react";
import { TransportPicker } from "@/components/TransportPicker";
import { useT } from "@/i18n/context";
import { portForTransport, webcastUrl } from "@/lib/webcast";
import type { Bitrate, StreamConfig, StreamFormat, Transport } from "@/types";

interface Props {
  config: StreamConfig;
  onChange: (config: StreamConfig) => void;
}

const BITRATES: Bitrate[] = [64, 128, 192, 320];
const FORMATS: { value: StreamFormat; label: string }[] = [
  { value: "mp3", label: "MP3" },
  { value: "aac", label: "AAC" },
];

export function ServerForm({ config, onChange }: Props) {
  const { t } = useT();
  const isWebcast = config.transport === "webcast";

  function update<K extends keyof StreamConfig>(key: K, value: StreamConfig[K]) {
    onChange({ ...config, [key]: value });
  }

  function changeTransport(transport: Transport) {
    onChange({
      ...config,
      transport,
      port: portForTransport(config.port, transport),
    });
  }

  return (
    <div className="flex flex-col gap-6">
      <Section title={t("settings.server")}>
        <TransportPicker value={config.transport} onChange={changeTransport} />

        <div className="grid grid-cols-3 gap-3">
          <Field label={t("settings.host")} className="col-span-2">
            <Input
              value={config.host}
              onChange={(e) => update("host", e.target.value)}
              placeholder={isWebcast ? "stream.example.com" : "localhost"}
            />
          </Field>
          <Field label={t("settings.port")}>
            <Input
              type="number"
              min={1}
              max={65535}
              value={config.port}
              onChange={(e) => update("port", Math.max(1, Math.min(65535, +e.target.value || 0)))}
              className="tabular-nums"
            />
          </Field>
        </div>

        <Field
          label={isWebcast ? t("settings.webdjPath") : t("settings.mount")}
          hint={isWebcast ? t("settings.webdjPathHint") : undefined}
        >
          <Input
            value={config.mount}
            onChange={(e) => update("mount", e.target.value)}
            placeholder={isWebcast ? "/webdj/my-station/" : "/aircast.mp3"}
          />
        </Field>

        {isWebcast && <EndpointPreview config={config} />}

        <div className="grid grid-cols-2 gap-3">
          <Field label={t("settings.username")}>
            <Input
              value={config.username}
              onChange={(e) => update("username", e.target.value)}
              placeholder={isWebcast ? "dj-name" : "source"}
            />
          </Field>
          <Field label={t("settings.password")}>
            <Input
              type="password"
              value={config.password}
              onChange={(e) => update("password", e.target.value)}
            />
          </Field>
        </div>
      </Section>

      <Section title={t("settings.encoding")}>
        <div className="grid grid-cols-2 gap-3">
          <Field label={t("settings.format")}>
            <Select
              value={config.format}
              onChange={(e) => update("format", e.target.value as StreamFormat)}
            >
              {FORMATS.map((f) => (
                <option key={f.value} value={f.value}>
                  {f.label}
                </option>
              ))}
            </Select>
          </Field>
          <Field label={t("settings.bitrate")}>
            <Select
              value={config.bitrate}
              onChange={(e) => update("bitrate", +e.target.value as Bitrate)}
              className="tabular-nums"
            >
              {BITRATES.map((b) => (
                <option key={b} value={b}>
                  {b} kbps
                </option>
              ))}
            </Select>
          </Field>
        </div>
      </Section>
    </div>
  );
}

/**
 * The address Aircast will actually dial.
 *
 * Host + port + path do not obviously add up to `wss://host/webdj/station/`,
 * and getting it wrong fails as a timeout rather than an error — so we show
 * the resolved URL instead of asking the user to picture it. The padlock also
 * makes it visible when a loopback host has dropped the connection to
 * plaintext, which is the only case where credentials are not encrypted.
 */
function EndpointPreview({ config }: { config: StreamConfig }) {
  const { t } = useT();
  const url = webcastUrl(config);
  const secure = url.startsWith("wss://");

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-xs text-zinc-500">{t("settings.endpointPreview")}</span>
      {/* 6px inner icon + 10px padding ≈ 16px outer radius. */}
      <div className="flex items-center gap-2.5 rounded-2xl bg-zinc-900/70 p-2.5 ring-1 ring-inset ring-zinc-800">
        <span
          className={[
            "flex h-6 w-6 shrink-0 items-center justify-center rounded-md",
            secure ? "bg-emerald-500/15 text-emerald-400" : "bg-amber-500/15 text-amber-400",
          ].join(" ")}
          title={secure ? t("settings.endpointSecure") : t("settings.endpointPlaintext")}
        >
          {secure ? (
            <Lock className="h-3.5 w-3.5" strokeWidth={2} />
          ) : (
            <ShieldAlert className="h-3.5 w-3.5" strokeWidth={2} />
          )}
        </span>
        <code className="min-w-0 overflow-x-auto font-mono text-xs whitespace-nowrap text-zinc-300">
          {url}
        </code>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-sm font-medium text-zinc-300">{title}</h3>
      <div className="flex flex-col gap-3">{children}</div>
    </section>
  );
}

function Field({
  label,
  children,
  className,
  hint,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
  hint?: string;
}) {
  return (
    <label className={`flex flex-col gap-1.5 ${className ?? ""}`}>
      <span className="text-xs text-zinc-500">{label}</span>
      {children}
      {hint && <span className="text-xs leading-snug text-pretty text-zinc-600">{hint}</span>}
    </label>
  );
}

function Input(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={[
        "rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600",
        "transition-[background-color,box-shadow] duration-150 ease-[cubic-bezier(0.2,0,0,1)]",
        "outline-none hover:bg-zinc-700/80 focus:bg-zinc-700",
        "focus:ring-2 focus:ring-rose-500/40 motion-reduce:transition-none",
        props.className ?? "",
      ].join(" ")}
    />
  );
}

function Select(props: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={[
        "cursor-pointer rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100",
        "transition-[background-color,box-shadow] duration-150 ease-[cubic-bezier(0.2,0,0,1)]",
        "outline-none hover:bg-zinc-700/80 focus:bg-zinc-700",
        "focus:ring-2 focus:ring-rose-500/40 motion-reduce:transition-none",
        props.className ?? "",
      ].join(" ")}
    />
  );
}
