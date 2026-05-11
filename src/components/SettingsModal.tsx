import { useEffect, useMemo, useRef, useState } from "react";
import {
  ClipboardCheck,
  ClipboardCopy,
  FileText,
  Folder,
  Globe,
  Link2,
  Plus,
  Send,
  Trash2,
  X,
} from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ServerForm } from "@/components/ServerForm";
import { usePresets } from "@/hooks/usePresets";
import { useT } from "@/i18n/context";
import type { LanguagePref } from "@/i18n";
import { api } from "@/lib/api";
import {
  DEFAULT_CONFIG,
  type MetadataMode,
  type MetadataSettings,
  type Preset,
  type Settings,
  type StreamConfig,
} from "@/types";

/// Tab to focus when the modal mounts. Used by in-app deep links (e.g. the
/// "edit" button on the live broadcast strip, the "Add a relay URL" CTA in
/// Relay mode, or the "Manage" link on the server destination card).
export type SettingsInitialSection = "servers" | "metadata" | "relay" | null;

type SettingsTab = "servers" | "relay" | "metadata" | "advanced";

interface Props {
  open: boolean;
  onClose: () => void;
  config: StreamConfig;
  onConfigChange: (config: StreamConfig) => void;
  settings: Settings;
  onSettingsChange: (settings: Settings) => void;
  onPasteLink?: (url: string) => boolean;
  initialSection?: SettingsInitialSection;
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
  initialSection,
}: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const { presets, refresh } = usePresets();
  const [pasteError, setPasteError] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<SettingsTab>("servers");

  // Jump to a specific tab when opened via an in-app deep link.
  useEffect(() => {
    if (!open) return;
    if (initialSection === "metadata") setActiveTab("metadata");
    else if (initialSection === "relay") setActiveTab("relay");
    else if (initialSection === "servers") setActiveTab("servers");
    // initialSection === null means "open wherever we already were".
  }, [open, initialSection]);
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

      {/* Tab bar — segments the modal into orthogonal concerns: which server
          to send to / where the source audio comes from in Relay mode /
          what listeners see / app-wide preferences. */}
      <nav className="flex shrink-0 items-center gap-1 border-b border-zinc-800 px-4">
        <TabButton
          active={activeTab === "servers"}
          onClick={() => setActiveTab("servers")}
        >
          {t("settings.tab.servers")}
        </TabButton>
        <TabButton
          active={activeTab === "relay"}
          onClick={() => setActiveTab("relay")}
        >
          {t("settings.tab.relay")}
        </TabButton>
        <TabButton
          active={activeTab === "metadata"}
          onClick={() => setActiveTab("metadata")}
        >
          {t("settings.tab.metadata")}
        </TabButton>
        <TabButton
          active={activeTab === "advanced"}
          onClick={() => setActiveTab("advanced")}
        >
          {t("settings.tab.advanced")}
        </TabButton>
      </nav>

      <div className="flex min-h-0 flex-1 gap-0 overflow-hidden">
        {activeTab === "servers" && (
          <>
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
                </div>
              )}
            </div>
          </>
        )}

        {activeTab === "relay" && (
          <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5">
            <RelaySection
              sources={settings.relaySources}
              onSourcesChange={(relaySources) =>
                onSettingsChange({ ...settings, relaySources })
              }
            />
          </div>
        )}

        {activeTab === "metadata" && (
          <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5">
            <MetadataSection
              settings={settings.metadata}
              onChange={(metadata) => onSettingsChange({ ...settings, metadata })}
            />
          </div>
        )}

        {activeTab === "advanced" && (
          <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5">
            <AdvancedSection
              settings={settings}
              onSettingsChange={onSettingsChange}
            />
          </div>
        )}
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

// ──────────────────────────────────────────────────────────────────────────────
// Metadata section: lets the user pick how the Icecast "now playing" title is
// composed (auto from music tags, static, or external file), and provides a
// "Test now" button that pushes the composed title to the server immediately.
//
// Variables shown in the help chips can be clicked to insert into the focused
// template input at the cursor position.
// ──────────────────────────────────────────────────────────────────────────────

const TEMPLATE_VARIABLES = [
  "{title}",
  "{artist}",
  "{album}",
  "{next_title}",
  "{next_artist}",
  "{show}",
  "{station}",
];

function MetadataSection({
  settings,
  onChange,
}: {
  settings: MetadataSettings;
  onChange: (next: MetadataSettings) => void;
}) {
  const { t } = useT();
  const templateRef = useRef<HTMLInputElement | null>(null);
  const micOverrideRef = useRef<HTMLInputElement | null>(null);
  const [focused, setFocused] = useState<"template" | "mic" | null>("template");
  const [testState, setTestState] = useState<"idle" | "ok" | "err">("idle");
  const [testError, setTestError] = useState<string | null>(null);

  function patch<K extends keyof MetadataSettings>(key: K, value: MetadataSettings[K]) {
    onChange({ ...settings, [key]: value });
  }

  function insertVariable(v: string) {
    const targetRef = focused === "mic" ? micOverrideRef : templateRef;
    const input = targetRef.current;
    if (!input) return;
    const start = input.selectionStart ?? input.value.length;
    const end = input.selectionEnd ?? input.value.length;
    const next = input.value.slice(0, start) + v + input.value.slice(end);
    if (focused === "mic") patch("micOverride", next);
    else patch("template", next);
    // restore caret after the inserted variable
    requestAnimationFrame(() => {
      input.focus();
      const caret = start + v.length;
      input.setSelectionRange(caret, caret);
    });
  }

  async function pickFile() {
    const result = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "Text", extensions: ["txt", "log", "md"] }, { name: "All", extensions: ["*"] }],
    });
    if (typeof result === "string") {
      patch("filePath", result);
    }
  }

  async function pushNow() {
    setTestState("idle");
    setTestError(null);
    try {
      await api.pushMetadataNow(null);
      setTestState("ok");
    } catch (e) {
      setTestState("err");
      setTestError(typeof e === "string" ? e : (e as Error).message ?? String(e));
    }
    window.setTimeout(() => setTestState("idle"), 2400);
  }

  return (
    <div className="mt-2 flex flex-col gap-5 rounded-lg bg-zinc-800/40 p-4">
      {/* Master toggle */}
      <label className="flex cursor-pointer items-center justify-between gap-3">
        <span className="text-sm text-zinc-200">{t("metadata.enabled")}</span>
        <input
          type="checkbox"
          checked={settings.enabled}
          onChange={(e) => patch("enabled", e.target.checked)}
          className="h-4 w-4 cursor-pointer accent-rose-500"
        />
      </label>

      {settings.enabled && (
        <>
          {/* Mode picker */}
          <div className="flex flex-col gap-2">
            <span className="text-xs uppercase tracking-wider text-zinc-500">
              {t("metadata.modeLabel")}
            </span>
            <div className="grid grid-cols-3 gap-2">
              {(["auto", "static", "file"] as MetadataMode[]).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => patch("mode", mode)}
                  className={[
                    "rounded-lg px-3 py-2.5 text-xs font-medium transition-colors",
                    settings.mode === mode
                      ? "bg-rose-500 text-white shadow-md shadow-rose-500/20"
                      : "bg-zinc-800 text-zinc-300 hover:bg-zinc-700",
                  ].join(" ")}
                >
                  {t(`metadata.mode.${mode}`)}
                </button>
              ))}
            </div>
            <p className="text-xs text-zinc-500">
              {t(`metadata.modeHint.${settings.mode}`)}
            </p>
          </div>

          {/* Mode-specific controls */}
          {settings.mode === "auto" && (
            <label className="flex flex-col gap-2">
              <span className="text-xs text-zinc-500">{t("metadata.template")}</span>
              <input
                ref={templateRef}
                type="text"
                value={settings.template}
                onFocus={() => setFocused("template")}
                onChange={(e) => patch("template", e.target.value)}
                placeholder="{artist} — {title}"
                className="rounded-lg bg-zinc-800 px-3.5 py-2.5 font-mono text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
              />
              <div className="flex flex-col gap-1.5">
                <span className="text-[11px] text-zinc-500">
                  {t("metadata.variablesHint")}
                </span>
                <div className="flex flex-wrap gap-1.5">
                  {TEMPLATE_VARIABLES.map((v) => (
                    <button
                      key={v}
                      type="button"
                      onClick={() => insertVariable(v)}
                      className="rounded-md bg-zinc-700/60 px-2 py-1 font-mono text-[11px] text-zinc-200 transition-colors hover:bg-zinc-600"
                    >
                      {v}
                    </button>
                  ))}
                </div>
              </div>
            </label>
          )}

          {settings.mode === "static" && (
            <label className="flex flex-col gap-1.5">
              <span className="text-xs text-zinc-500">{t("metadata.staticText")}</span>
              <input
                type="text"
                value={settings.staticText}
                onChange={(e) => patch("staticText", e.target.value)}
                placeholder={t("metadata.staticPlaceholder")}
                className="rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
              />
            </label>
          )}

          {settings.mode === "file" && (
            <div className="flex flex-col gap-3">
              <label className="flex flex-col gap-1.5">
                <span className="text-xs text-zinc-500">{t("metadata.filePath")}</span>
                <div className="flex items-center gap-2">
                  <div className="flex flex-1 items-center gap-2 truncate rounded-lg bg-zinc-800 px-3.5 py-2.5">
                    <FileText className="h-3.5 w-3.5 shrink-0 text-zinc-500" />
                    <span className="truncate text-sm text-zinc-200">
                      {settings.filePath || (
                        <span className="text-zinc-500">{t("metadata.fileNone")}</span>
                      )}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={pickFile}
                    className="flex items-center gap-1.5 rounded-lg bg-zinc-700 px-3 py-2.5 text-xs font-medium text-zinc-200 hover:bg-zinc-600"
                  >
                    <Folder className="h-3.5 w-3.5" />
                    <span>{t("metadata.fileBrowse")}</span>
                  </button>
                </div>
              </label>
              <label className="flex items-center justify-between gap-3">
                <span className="text-sm text-zinc-300">{t("metadata.filePoll")}</span>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    min={1}
                    max={60}
                    value={settings.filePollSecs}
                    onChange={(e) =>
                      patch(
                        "filePollSecs",
                        Math.max(1, Math.min(60, +e.target.value || 5)),
                      )
                    }
                    className="w-20 rounded-lg bg-zinc-800 px-3 py-2 text-right text-sm tabular-nums text-zinc-100 outline-none focus:ring-2 focus:ring-rose-500/40"
                  />
                  <span className="text-xs text-zinc-500">
                    {t("settings.secondsShort")}
                  </span>
                </div>
              </label>
            </div>
          )}

          {/* Mic override (cross-cutting) */}
          <div className="flex flex-col gap-2 border-t border-zinc-800 pt-4">
            <label className="flex flex-col gap-1.5">
              <span className="text-xs text-zinc-500">{t("metadata.micOverride")}</span>
              <input
                ref={micOverrideRef}
                type="text"
                value={settings.micOverride}
                onFocus={() => setFocused("mic")}
                onChange={(e) => patch("micOverride", e.target.value)}
                placeholder={t("metadata.micOverridePlaceholder")}
                className="rounded-lg bg-zinc-800 px-3.5 py-2.5 font-mono text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
              />
              <span className="text-[11px] text-zinc-500">
                {t("metadata.micOverrideHint")}
              </span>
            </label>
          </div>

          {/* Identity */}
          <div className="flex flex-col gap-2 border-t border-zinc-800 pt-4">
            <span className="text-xs uppercase tracking-wider text-zinc-500">
              {t("metadata.identityLabel")}
            </span>
            <div className="grid grid-cols-2 gap-2">
              <label className="flex flex-col gap-1">
                <span className="text-[11px] text-zinc-500">
                  {t("metadata.stationName")}
                </span>
                <input
                  type="text"
                  value={settings.stationName}
                  onChange={(e) => patch("stationName", e.target.value)}
                  placeholder="Radio XYZ"
                  className="rounded-lg bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-[11px] text-zinc-500">
                  {t("metadata.showName")}
                </span>
                <input
                  type="text"
                  value={settings.showName}
                  onChange={(e) => patch("showName", e.target.value)}
                  placeholder={t("metadata.showPlaceholder")}
                  className="rounded-lg bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
                />
              </label>
            </div>
          </div>

          {/* Test push */}
          <div className="flex flex-col gap-2 border-t border-zinc-800 pt-4">
            <button
              type="button"
              onClick={pushNow}
              className={[
                "flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-medium transition-colors",
                testState === "ok"
                  ? "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
                  : testState === "err"
                    ? "bg-red-500/15 text-red-300 ring-1 ring-red-500/30"
                    : "bg-zinc-800 text-zinc-200 hover:bg-zinc-700",
              ].join(" ")}
            >
              <Send className="h-4 w-4" />
              <span>
                {testState === "ok"
                  ? t("metadata.testOk")
                  : testState === "err"
                    ? t("metadata.testErr")
                    : t("metadata.testPush")}
              </span>
            </button>
            {testState === "err" && testError && (
              <span className="text-[11px] text-red-300/80">{testError}</span>
            )}
            <span className="text-[11px] text-zinc-500">{t("metadata.testHint")}</span>
          </div>
        </>
      )}
    </div>
  );
}

// ──────────────────────────────────────────────────────────────────────────────
// Relay sources section: lets the user manage named upstream stream URLs
// used by the Relay mode, and toggle which top-level modes appear in the
// header switch. Saved URLs survive across launches.
// ──────────────────────────────────────────────────────────────────────────────

function RelaySection({
  sources,
  onSourcesChange,
}: {
  sources: import("@/types").RelaySource[];
  onSourcesChange: (sources: import("@/types").RelaySource[]) => void;
}) {
  const { t } = useT();
  // Form state for adding/editing a source. `editingName === null` means
  // "creating a new entry"; otherwise we're editing the existing one in
  // place. The form is collapsed into a single button row by default to
  // keep the section visually compact.
  const [editingName, setEditingName] = useState<string | null>(null);
  const [draftName, setDraftName] = useState("");
  const [draftUrl, setDraftUrl] = useState("");
  const [formOpen, setFormOpen] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  function startCreate() {
    setEditingName(null);
    setDraftName("");
    setDraftUrl("");
    setFormError(null);
    setFormOpen(true);
  }

  function startEdit(s: import("@/types").RelaySource) {
    setEditingName(s.name);
    setDraftName(s.name);
    setDraftUrl(s.url);
    setFormError(null);
    setFormOpen(true);
  }

  function cancelForm() {
    setFormOpen(false);
    setFormError(null);
  }

  async function commitForm() {
    const name = draftName.trim();
    const url = draftUrl.trim();
    if (!name) {
      setFormError(t("relay.errors.nameRequired"));
      return;
    }
    if (!url) {
      setFormError(t("relay.errors.urlRequired"));
      return;
    }
    if (
      editingName === null &&
      sources.some((s) => s.name === name)
    ) {
      setFormError(t("relay.errors.nameTaken"));
      return;
    }
    if (editingName !== null && editingName !== name) {
      // Rename: persist via backend, also update local list.
      try {
        await api.renameRelaySource(editingName, name);
      } catch (e) {
        setFormError(String(e));
        return;
      }
    }
    try {
      await api.upsertRelaySource({ name, url });
    } catch (e) {
      setFormError(String(e));
      return;
    }
    // Mirror to parent settings — keep the local in-memory list in sync.
    const without = sources.filter(
      (s) => s.name !== (editingName ?? name),
    );
    onSourcesChange([...without, { name, url }].sort((a, b) => a.name.localeCompare(b.name)));
    setFormOpen(false);
  }

  async function removeSource(name: string) {
    try {
      await api.deleteRelaySource(name);
    } catch {
      // ignore — backend already logged it
    }
    onSourcesChange(sources.filter((s) => s.name !== name));
  }

  return (
    <div className="mt-2 flex flex-col gap-5 rounded-lg bg-zinc-800/40 p-4">
      {/* Sources list */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="text-xs uppercase tracking-wider text-zinc-500">
            {t("relay.sourcesLabel")}
          </span>
          <button
            type="button"
            onClick={startCreate}
            className="flex items-center gap-1 rounded-full bg-zinc-700 px-2.5 py-1 text-[11px] font-semibold text-zinc-100 hover:bg-zinc-600"
          >
            <Plus className="h-3 w-3" />
            {t("relay.newSource")}
          </button>
        </div>
        {sources.length === 0 && !formOpen && (
          <p className="rounded-lg bg-zinc-900/50 px-3 py-4 text-center text-xs text-zinc-500">
            {t("relay.emptyHint")}
          </p>
        )}
        {sources.length > 0 && (
          <ul className="flex flex-col gap-1.5">
            {sources.map((s) => (
              <li
                key={s.name}
                className="flex items-center justify-between gap-3 rounded-lg bg-zinc-900/60 px-3 py-2.5 ring-1 ring-zinc-800/80"
              >
                <button
                  type="button"
                  onClick={() => startEdit(s)}
                  className="flex min-w-0 flex-1 flex-col items-start text-left"
                >
                  <span className="truncate text-sm text-zinc-100">{s.name}</span>
                  <span className="truncate text-[11px] text-zinc-500">{s.url}</span>
                </button>
                <button
                  type="button"
                  onClick={() => removeSource(s.name)}
                  title={t("relay.deleteHint")}
                  className="rounded-full p-1 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-rose-300"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </li>
            ))}
          </ul>
        )}

        {/* Inline form for create/edit */}
        {formOpen && (
          <div className="mt-1 flex flex-col gap-2 rounded-lg bg-zinc-900/60 p-3 ring-1 ring-zinc-800">
            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-zinc-500">{t("relay.nameField")}</span>
              <input
                type="text"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                placeholder="France Inter"
                className="rounded-lg bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none focus:ring-2 focus:ring-rose-500/40"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-zinc-500">{t("relay.urlField")}</span>
              <input
                type="text"
                value={draftUrl}
                onChange={(e) => setDraftUrl(e.target.value)}
                placeholder="https://stream.example.com/live.mp3"
                className="rounded-lg bg-zinc-800 px-3 py-2 font-mono text-xs text-zinc-100 outline-none focus:ring-2 focus:ring-rose-500/40"
              />
            </label>
            {formError && (
              <span className="text-[11px] text-red-300">{formError}</span>
            )}
            <div className="mt-1 flex items-center justify-end gap-2">
              <button
                type="button"
                onClick={cancelForm}
                className="rounded-lg px-3 py-1.5 text-xs font-medium text-zinc-400 hover:text-zinc-200"
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                onClick={() => void commitForm()}
                className="rounded-lg bg-rose-500 px-4 py-1.5 text-xs font-semibold text-white hover:bg-rose-400"
              >
                {t("common.save")}
              </button>
            </div>
          </div>
        )}
      </div>

    </div>
  );
}

// ──────────────────────────────────────────────────────────────────────────────
// Advanced tab: ducking, crossfade, reconnect delay, mode visibility toggles,
// diagnostic bundle. App-wide preferences, never per-server.
// ──────────────────────────────────────────────────────────────────────────────

function AdvancedSection({
  settings,
  onSettingsChange,
}: {
  settings: Settings;
  onSettingsChange: (s: Settings) => void;
}) {
  const { t } = useT();

  function toggleMode(key: keyof import("@/types").EnabledModes) {
    const next = { ...settings.enabledModes, [key]: !settings.enabledModes[key] };
    // At least one mode must remain visible or the header switch becomes
    // useless. Silently bounce the click.
    if (!next.simple && !next.studio && !next.relay) return;
    onSettingsChange({ ...settings, enabledModes: next });
  }

  return (
    <div className="flex flex-col gap-6">
      {/* Audio behaviour */}
      <section className="flex flex-col gap-4 rounded-lg bg-zinc-800/40 p-4">
        <span className="text-xs uppercase tracking-wider text-zinc-500">
          {t("settings.audioBehavior")}
        </span>

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
                musicVolumeWhenMicOpen: Math.max(0, Math.min(1, +e.target.value / 100)),
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
                reconnectIntervalSeconds: Math.max(0, Math.min(3600, +e.target.value || 0)),
              })
            }
            className="rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 outline-none hover:bg-zinc-700/80 focus:bg-zinc-700 focus:ring-2 focus:ring-rose-500/40"
          />
        </label>
      </section>

      {/* Interface — modes available */}
      <section className="flex flex-col gap-3 rounded-lg bg-zinc-800/40 p-4">
        <div className="flex flex-col gap-1">
          <span className="text-xs uppercase tracking-wider text-zinc-500">
            {t("relay.enabledModesLabel")}
          </span>
          <p className="text-[11px] text-zinc-500">{t("relay.enabledModesHint")}</p>
        </div>
        <div className="grid grid-cols-3 gap-2">
          {(["studio", "simple", "relay"] as const).map((m) => {
            const active = settings.enabledModes[m];
            return (
              <button
                key={m}
                type="button"
                onClick={() => toggleMode(m)}
                className={[
                  "rounded-lg px-3 py-2.5 text-xs font-medium transition-colors",
                  active
                    ? "bg-rose-500 text-white shadow-md shadow-rose-500/20"
                    : "bg-zinc-900/60 text-zinc-500 hover:bg-zinc-800/80",
                ].join(" ")}
              >
                {t(`mode.${m}`)}
              </button>
            );
          })}
        </div>
      </section>

      {/* Diagnostic */}
      <section className="flex flex-col gap-2 rounded-lg bg-zinc-800/40 p-4">
        <span className="text-xs uppercase tracking-wider text-zinc-500">
          {t("settings.diagnosticTitle")}
        </span>
        <p className="text-xs text-zinc-500">{t("settings.diagnosticHint")}</p>
        <DiagnosticCopyButton />
      </section>
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "relative px-4 py-3 text-sm font-medium transition-colors",
        active ? "text-zinc-100" : "text-zinc-500 hover:text-zinc-300",
      ].join(" ")}
    >
      {children}
      {/* Underline indicator for the active tab. Rose accent matches the
          ModeSwitch sliding indicator so the two controls feel related. */}
      <span
        aria-hidden
        className={[
          "absolute inset-x-3 -bottom-px h-0.5 rounded-full transition-opacity",
          active ? "bg-rose-500 opacity-100" : "opacity-0",
        ].join(" ")}
      />
    </button>
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
