import { useT } from "@/i18n/context";
import type { Bitrate, StreamConfig, StreamFormat } from "@/types";

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
  function update<K extends keyof StreamConfig>(key: K, value: StreamConfig[K]) {
    onChange({ ...config, [key]: value });
  }

  return (
    <div className="flex flex-col gap-5">
      <Section title={t("settings.server")}>
        <div className="grid grid-cols-3 gap-3">
          <Field label={t("settings.host")} className="col-span-2">
            <Input
              value={config.host}
              onChange={(e) => update("host", e.target.value)}
              placeholder="localhost"
            />
          </Field>
          <Field label={t("settings.port")}>
            <Input
              type="number"
              min={1}
              max={65535}
              value={config.port}
              onChange={(e) => update("port", Math.max(1, Math.min(65535, +e.target.value || 0)))}
            />
          </Field>
        </div>
        <Field label={t("settings.mount")}>
          <Input
            value={config.mount}
            onChange={(e) => update("mount", e.target.value)}
            placeholder="/aircast.mp3"
          />
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label={t("settings.username")}>
            <Input
              value={config.username}
              onChange={(e) => update("username", e.target.value)}
              placeholder="source"
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
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <label className={`flex flex-col gap-1.5 ${className ?? ""}`}>
      <span className="text-xs text-zinc-500">{label}</span>
      {children}
    </label>
  );
}

function Input(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={[
        "rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600",
        "outline-none transition-colors hover:bg-zinc-700/80 focus:bg-zinc-700",
        "focus:ring-2 focus:ring-rose-500/40",
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
        "outline-none transition-colors hover:bg-zinc-700/80 focus:bg-zinc-700",
        "focus:ring-2 focus:ring-rose-500/40",
        props.className ?? "",
      ].join(" ")}
    />
  );
}
