import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, ClipboardCheck, ClipboardCopy, Globe, Link2, Plus, Trash2, X } from "lucide-react";
import { ServerForm } from "@/components/ServerForm";
import { usePresets } from "@/hooks/usePresets";
import { useT } from "@/i18n/context";
import type { LanguagePref } from "@/i18n";
import { api } from "@/lib/api";
import { DEFAULT_CONFIG, type Preset, type Settings, type StreamConfig } from "@/types";

interface Props {
  open: boolean;
  onClose: () => void;
  config: StreamConfig;
  onConfigChange: (config: StreamConfig) => void;
  settings: Settings;
  onSettingsChange: (settings: Settings) => void;
  onPasteLink?: (url: string) => boolean;
}

const SAVE_DEBOUNCE_MS = 400;

export function SettingsModal({
  open,
  onClose,
  config,
  onConfigChange,
  settings,
  onSettingsChange,
  onPasteLink,
}: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const { presets, refresh } = usePresets();
  const [pasteError, setPasteError] = useState<string | null>(null);

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [activeName, setActiveNameState] = useState<string | null>(
    settings.activePreset ?? null,
  );
  const [nameDraft, setNameDraft] = useState<string>("");
  const [nameError, setNameError] = useState<string | null>(null);

  // Keep activeName in sync with persisted settings.
  useEffect(() => {
    setActiveNameState(settings.activePreset ?? null);
  }, [settings.activePreset]);

  // Find the active preset object.
  const activePreset = useMemo(
    () => (activeName ? presets.find((p) => p.name === activeName) ?? null : null),
    [presets, activeName],
  );

  // Reset name draft when active preset changes.
  useEffect(() => {
    setNameDraft(activePreset?.name ?? "");
    setNameError(null);
  }, [activePreset?.name]);

  // Open/close the native <dialog>.
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);

  // First open with no presets: bootstrap a default one from current config.
  useEffect(() => {
    if (!open) return;
    if (presets.length > 0) return;
    const bootName = uniqueName("My server", presets);
    void (async () => {
      const cfg: StreamConfig = {
        ...DEFAULT_CONFIG,
        ...config,
        deviceId: config.deviceId,
      };
      await api.savePreset(bootName, cfg);
      await refresh();
      onConfigChange(cfg);
      onSettingsChange({ ...settings, activePreset: bootName });
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presets.length]);

  // Auto-save edits (config) to the active preset, debounced.
  const lastSavedRef = useRef<{ name: string; config: StreamConfig } | null>(null);
  useEffect(() => {
    if (!activeName) return;
    if (!activePreset) return;
    if (sameConfig(lastSavedRef.current?.config, config)) return;
    if (lastSavedRef.current?.name !== activeName) {
      lastSavedRef.current = { name: activeName, config };
    }
    const handle = setTimeout(() => {
      void api.savePreset(activeName, config).then(() => {
        lastSavedRef.current = { name: activeName, config };
        void refresh();
      });
    }, SAVE_DEBOUNCE_MS);
    return () => clearTimeout(handle);
  }, [config, activeName, activePreset, refresh]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) onClose();
  }

  function selectPreset(p: Preset) {
    setActiveNameState(p.name);
    onSettingsChange({ ...settings, activePreset: p.name });
    onConfigChange({ ...p.config, deviceId: config.deviceId });
  }

  async function handleNew() {
    const name = uniqueName("New server", presets);
    const cfg: StreamConfig = { ...DEFAULT_CONFIG, deviceId: config.deviceId };
    await api.savePreset(name, cfg);
    await refresh();
    setActiveNameState(name);
    onSettingsChange({ ...settings, activePreset: name });
    onConfigChange(cfg);
  }

  async function handlePasteLink() {
    setPasteError(null);
    if (!onPasteLink) return;
    try {
      const text = await navigator.clipboard.readText();
      const trimmed = text.trim();
      if (!trimmed) {
        setPasteError(t("link.pasteEmpty"));
        return;
      }
      const ok = onPasteLink(trimmed);
      if (!ok) setPasteError(t("link.pasteInvalid"));
    } catch (e) {
      setPasteError(String(e));
    }
  }

  async function handleDelete(p: Preset) {
    await api.deletePreset(p.name);
    await refresh();
    if (activeName === p.name) {
      const remaining = presets.filter((x) => x.name !== p.name);
      const next = remaining[0] ?? null;
      setActiveNameState(next?.name ?? null);
      onSettingsChange({ ...settings, activePreset: next?.name ?? null });
      if (next) onConfigChange({ ...next.config, deviceId: config.deviceId });
    }
  }

  async function commitRename() {
    if (!activeName) return;
    const trimmed = nameDraft.trim();
    if (!trimmed || trimmed === activeName) {
      setNameDraft(activeName);
      setNameError(null);
      return;
    }
    if (presets.some((p) => p.name === trimmed)) {
      setNameError(t("settings.nameTaken"));
      return;
    }
    setNameError(null);
    try {
      await api.renamePreset(activeName, trimmed);
      await refresh();
      setActiveNameState(trimmed);
      onSettingsChange({ ...settings, activePreset: trimmed });
      lastSavedRef.current = { name: trimmed, config };
    } catch (e) {
      setNameError(String(e));
      setNameDraft(activeName);
    }
  }

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      onClick={handleBackdropClick}
      className="fixed inset-0 m-auto h-[640px] max-h-[85vh] w-full max-w-3xl rounded-2xl bg-zinc-900 p-0 text-zinc-100 shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm open:flex open:flex-col"
    >
      <header className="flex items-center justify-between px-6 py-4">
        <h2 className="text-base font-semibold tracking-tight">{t("settings.title")}</h2>
        <button
          type="button"
          onClick={onClose}
          className="rounded-full p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        >
          <X className="h-4 w-4" />
        </button>
      </header>

      <div className="flex min-h-0 flex-1 gap-0 overflow-hidden">
        <ServersSidebar
          presets={presets}
          activeName={activeName}
          onSelect={selectPreset}
          onNew={handleNew}
          onDelete={handleDelete}
          onPasteLink={onPasteLink ? handlePasteLink : undefined}
          pasteError={pasteError}
          dismissPasteError={() => setPasteError(null)}
        />

        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5">
          {!activePreset ? (
            <EmptyEditor onNew={handleNew} hasPresets={presets.length > 0} />
          ) : (
            <div className="flex flex-col gap-5">
              <div className="flex flex-col gap-1">
                <input
                  value={nameDraft}
                  onChange={(e) => {
                    setNameDraft(e.target.value);
                    setNameError(null);
                  }}
                  onBlur={() => void commitRename()}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      (e.target as HTMLInputElement).blur();
                    }
                  }}
                  placeholder={t("settings.serverNamePlaceholder")}
                  className="-mx-1 rounded-md bg-transparent px-1 py-0.5 text-xl font-semibold tracking-tight text-zinc-100 outline-none transition-colors placeholder:text-zinc-600 hover:bg-zinc-800/40 focus:bg-zinc-800/60"
                />
                {nameError && <p className="text-xs text-rose-400">{nameError}</p>}
              </div>

              <ServerForm config={config} onChange={onConfigChange} />

              <section>
                <button
                  type="button"
                  onClick={() => setShowAdvanced((v) => !v)}
                  className="flex w-full items-center justify-between rounded-lg px-1 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-100"
                >
                  <span>{t("settings.advanced")}</span>
                  <ChevronDown
                    className={`h-4 w-4 transition-transform ${showAdvanced ? "rotate-180" : ""}`}
                  />
                </button>
                {showAdvanced && (
                  <div className="mt-2 flex flex-col gap-4 rounded-lg bg-zinc-800/40 p-4">
                    <label className="flex flex-col gap-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-zinc-300">{t("settings.ducking")}</span>
                        <span className="rounded-full bg-zinc-800 px-2 py-0.5 font-mono text-xs tabular-nums text-zinc-100">
                          {Math.round(settings.musicVolumeWhenMicOpen * 100)}%
                        </span>
                      </div>
                      <input
                        type="range"
                        min={0}
                        max={100}
                        step={5}
                        value={Math.round(settings.musicVolumeWhenMicOpen * 100)}
                        onChange={(e) =>
                          onSettingsChange({
                            ...settings,
                            musicVolumeWhenMicOpen: Math.max(
                              0,
                              Math.min(1, +e.target.value / 100),
                            ),
                          })
                        }
                        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-zinc-700 accent-rose-500"
                      />
                      <span className="text-xs text-zinc-500">{t("settings.duckingHint")}</span>
                    </label>

                    <label className="flex flex-col gap-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-zinc-300">{t("settings.crossfade")}</span>
                        <span className="rounded-full bg-zinc-800 px-2 py-0.5 font-mono text-xs tabular-nums text-zinc-100">
                          {settings.crossfadeSeconds === 0
                            ? t("settings.off")
                            : `${settings.crossfadeSeconds.toFixed(1)} ${t("settings.secondsShort")}`}
                        </span>
                      </div>
                      <input
                        type="range"
                        min={0}
                        max={10}
                        step={0.5}
                        value={settings.crossfadeSeconds}
                        onChange={(e) =>
                          onSettingsChange({
                            ...settings,
                            crossfadeSeconds: Math.max(0, Math.min(10, +e.target.value)),
                          })
                        }
                        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-zinc-700 accent-rose-500"
                      />
                      <span className="text-xs text-zinc-500">{t("settings.crossfadeHint")}</span>
                    </label>

                    <label className="flex flex-col gap-1.5">
                      <span className="text-xs text-zinc-500">{t("settings.reconnect")}</span>
                      <input
                        type="number"
                        min={0}
                        max={3600}
                        value={settings.reconnectIntervalSeconds}
                        onChange={(e) =>
                          onSettingsChange({
                            ...settings,
                            reconnectIntervalSeconds: Math.max(
                              0,
                              Math.min(3600, +e.target.value || 0),
                            ),
                          })
                        }
                        className="rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
                      />
                    </label>

                    <div className="mt-1 flex flex-col gap-1.5 border-t border-zinc-800 pt-4">
                      <span className="text-xs text-zinc-500">
                        {t("settings.diagnosticHint")}
                      </span>
                      <DiagnosticCopyButton />
                    </div>
                  </div>
                )}
              </section>
            </div>
          )}
        </div>
      </div>

      <footer className="flex items-center justify-between gap-4 border-t border-zinc-800 px-6 py-4">
        <div className="flex items-center gap-3">
          <Globe className="h-4 w-4 text-zinc-500" />
          <LanguagePicker
            value={(settings.language as LanguagePref) || "auto"}
            onChange={(pref) => onSettingsChange({ ...settings, language: pref })}
          />
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-zinc-500">{t("settings.savedAuto")}</span>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg bg-rose-500 px-5 py-2 text-sm font-medium text-white hover:bg-rose-400"
          >
            {t("common.done")}
          </button>
        </div>
      </footer>
    </dialog>
  );
}

function ServersSidebar({
  presets,
  activeName,
  onSelect,
  onNew,
  onDelete,
  onPasteLink,
  pasteError,
  dismissPasteError,
}: {
  presets: Preset[];
  activeName: string | null;
  onSelect: (p: Preset) => void;
  onNew: () => void;
  onDelete: (p: Preset) => void;
  onPasteLink?: () => void;
  pasteError: string | null;
  dismissPasteError: () => void;
}) {
  const { t } = useT();
  return (
    <aside className="flex w-56 shrink-0 flex-col gap-2 border-r border-zinc-800/70 bg-zinc-950/50 px-3 py-4">
      <div className="flex items-center justify-between px-2">
        <h3 className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
          {t("settings.serversTitle")}
        </h3>
        <div className="flex items-center gap-1">
          {onPasteLink && (
            <button
              type="button"
              onClick={onPasteLink}
              title={t("link.pasteHint")}
              className="rounded-full bg-rose-500/15 p-1.5 text-rose-300 hover:bg-rose-500/25"
            >
              <Link2 className="h-3 w-3" />
            </button>
          )}
          <button
            type="button"
            onClick={onNew}
            title={t("settings.newServer")}
            className="flex items-center gap-1 rounded-full bg-zinc-800 px-2 py-1 text-[11px] font-semibold text-zinc-200 hover:bg-zinc-700"
          >
            <Plus className="h-3 w-3" />
            {t("settings.newServer")}
          </button>
        </div>
      </div>
      {pasteError && (
        <button
          type="button"
          onClick={dismissPasteError}
          className="mx-2 rounded-md bg-rose-500/15 px-2 py-1 text-left text-[10px] text-rose-300 hover:bg-rose-500/20"
        >
          {pasteError}
        </button>
      )}
      <div className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
        {presets.length === 0 && (
          <div className="px-2 py-3 text-xs text-zinc-600">{t("settings.empty")}</div>
        )}
        {presets.map((p) => (
          <PresetItem
            key={p.name}
            preset={p}
            active={p.name === activeName}
            onSelect={() => onSelect(p)}
            onDelete={() => onDelete(p)}
          />
        ))}
      </div>
    </aside>
  );
}

function PresetItem({
  preset,
  active,
  onSelect,
  onDelete,
}: {
  preset: Preset;
  active: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  const { t } = useT();
  return (
    <div
      className={[
        "group flex items-center gap-2 rounded-md px-2 py-2 transition-colors",
        active ? "bg-zinc-800 text-zinc-100" : "text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-100",
      ].join(" ")}
    >
      <button
        type="button"
        onClick={onSelect}
        className="flex min-w-0 flex-1 items-center gap-2 text-left"
      >
        <span
          className={`h-1.5 w-1.5 shrink-0 rounded-full ${active ? "bg-rose-500" : "bg-zinc-700"}`}
        />
        <span className="truncate text-sm font-medium">{preset.name}</span>
      </button>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        title={t("settings.deleteServer")}
        className="hidden rounded-full p-1 text-zinc-500 hover:bg-rose-500 hover:text-white group-hover:block"
      >
        <Trash2 className="h-3 w-3" />
      </button>
    </div>
  );
}

function EmptyEditor({ onNew, hasPresets }: { onNew: () => void; hasPresets: boolean }) {
  const { t } = useT();
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <div className="rounded-full bg-zinc-800 p-3 text-zinc-400">
        <Plus className="h-6 w-6" />
      </div>
      <p className="max-w-xs text-sm text-zinc-400">
        {hasPresets ? t("settings.selectHint") : t("settings.emptyHint")}
      </p>
      {!hasPresets && (
        <button
          type="button"
          onClick={onNew}
          className="rounded-full bg-rose-500 px-4 py-2 text-sm font-semibold text-white hover:bg-rose-400"
        >
          {t("settings.newServer")}
        </button>
      )}
    </div>
  );
}

function LanguagePicker({
  value,
  onChange,
}: {
  value: LanguagePref;
  onChange: (pref: LanguagePref) => void;
}) {
  const { t } = useT();
  const options: { value: LanguagePref; label: string }[] = [
    { value: "auto", label: t("settings.languageAuto") },
    { value: "en", label: t("settings.languageEn") },
    { value: "fr", label: t("settings.languageFr") },
  ];
  return (
    <div className="flex items-center rounded-full bg-zinc-800 p-0.5">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => onChange(opt.value)}
          className={[
            "rounded-full px-3 py-1 text-xs font-medium transition-colors",
            value === opt.value
              ? "bg-zinc-700 text-zinc-100"
              : "text-zinc-400 hover:text-zinc-200",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

function DiagnosticCopyButton() {
  const { t } = useT();
  const [state, setState] = useState<"idle" | "copied" | "error">("idle");
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    };
  }, []);

  async function handleClick() {
    try {
      const bundle = await api.getDiagnosticBundle();
      await navigator.clipboard.writeText(bundle);
      setState("copied");
    } catch {
      setState("error");
    }
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => setState("idle"), 2200);
  }

  const Icon = state === "copied" ? ClipboardCheck : ClipboardCopy;
  const label =
    state === "copied"
      ? t("settings.diagnosticCopied")
      : state === "error"
        ? t("settings.diagnosticFailed")
        : t("settings.diagnosticCopy");

  return (
    <button
      type="button"
      onClick={handleClick}
      className={[
        "flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-medium transition-colors",
        state === "copied"
          ? "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
          : state === "error"
            ? "bg-red-500/15 text-red-300 ring-1 ring-red-500/30"
            : "bg-zinc-800 text-zinc-200 hover:bg-zinc-700",
      ].join(" ")}
    >
      <Icon className="h-4 w-4" />
      <span>{label}</span>
    </button>
  );
}

function uniqueName(base: string, presets: Preset[]): string {
  if (!presets.some((p) => p.name === base)) return base;
  let i = 2;
  while (presets.some((p) => p.name === `${base} ${i}`)) i++;
  return `${base} ${i}`;
}

function sameConfig(a: StreamConfig | undefined, b: StreamConfig | undefined): boolean {
  if (!a || !b) return false;
  return (
    a.host === b.host &&
    a.port === b.port &&
    a.mount === b.mount &&
    a.username === b.username &&
    a.password === b.password &&
    a.bitrate === b.bitrate &&
    a.format === b.format
  );
}
