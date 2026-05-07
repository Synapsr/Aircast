import { useCallback, useEffect, useMemo, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Mic, Radio, Settings as SettingsIcon, SlidersHorizontal } from "lucide-react";
import { AddServerFromLinkModal } from "@/components/AddServerFromLinkModal";
import { DevicePill } from "@/components/DevicePill";
import { ErrorBanner } from "@/components/ErrorBanner";
import { ErrorDialog } from "@/components/ErrorDialog";
import { SimpleMode } from "@/components/SimpleMode";
import { SettingsModal } from "@/components/SettingsModal";
import { StudioMode } from "@/components/studio/StudioMode";
import { StatusBar } from "@/components/studio/StatusBar";
import { api } from "@/lib/api";
import { parseServerLink, uniquePresetName, type ParsedServerLink } from "@/lib/deeplink";
import { validateStreamConfig } from "@/lib/validation";
import { useCurrentConfig } from "@/hooks/useCurrentConfig";
import { useDeepLink } from "@/hooks/useDeepLink";
import { useSettings } from "@/hooks/useSettings";
import { useStreamStatus } from "@/hooks/useStreamStatus";
import { useVuLevel } from "@/hooks/useVuLevel";
import { useMode } from "@/hooks/useMode";
import { useMicOpen } from "@/hooks/useMicOpen";
import { usePresets } from "@/hooks/usePresets";
import { LocaleProvider, useT } from "@/i18n/context";
import type { LanguagePref } from "@/i18n";
import type { AppMode, StreamStatus } from "@/types";

export default function App() {
  const { settings, update: updateSettings } = useSettings();

  const setPref = useCallback(
    (pref: LanguagePref) => {
      void updateSettings({ ...settings, language: pref });
    },
    [settings, updateSettings],
  );

  const pref = (settings.language as LanguagePref) || "auto";

  return (
    <LocaleProvider pref={pref} setPref={setPref}>
      <Shell settings={settings} updateSettings={updateSettings} />
    </LocaleProvider>
  );
}

interface ShellProps {
  settings: ReturnType<typeof useSettings>["settings"];
  updateSettings: ReturnType<typeof useSettings>["update"];
}

function Shell({ settings, updateSettings }: ShellProps) {
  const { t } = useT();
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const { config, update: updateConfig, loaded: configLoaded } = useCurrentConfig(deviceId);
  const { presets, refresh: refreshPresets } = usePresets();
  const { mode, setMode } = useMode();
  const { open: micOpen } = useMicOpen();
  const status = useStreamStatus();
  const level = useVuLevel();
  const { url: deepLinkUrl, clear: clearDeepLink } = useDeepLink();
  const [pendingLink, setPendingLink] = useState<ParsedServerLink | null>(null);
  const [pendingFinalName, setPendingFinalName] = useState<string | null>(null);

  // Parse incoming deep-link URLs and prepare a confirmation modal.
  useEffect(() => {
    if (!deepLinkUrl) return;
    const parsed = parseServerLink(deepLinkUrl, deviceId ?? "");
    if (parsed) {
      const finalName = uniquePresetName(
        parsed.name,
        presets.map((p) => p.name),
      );
      setPendingLink(parsed);
      setPendingFinalName(finalName);
    }
    clearDeepLink();
  }, [deepLinkUrl, deviceId, presets, clearDeepLink]);

  // Manual paste path: same flow as the deep-link reception, but driven by
  // the user copying an `aircast://…` URL and clicking "Paste link" in Setup.
  // Returns true when the URL parsed successfully, so the modal can show a
  // helpful error otherwise.
  const handlePasteLink = (url: string): boolean => {
    const parsed = parseServerLink(url, deviceId ?? "");
    if (!parsed) return false;
    const finalName = uniquePresetName(parsed.name, presets.map((p) => p.name));
    setPendingLink(parsed);
    setPendingFinalName(finalName);
    return true;
  };

  async function handleConfirmLink() {
    if (!pendingLink || !pendingFinalName) return;
    try {
      const newConfig = { ...pendingLink.config, deviceId: deviceId ?? "" };
      await api.savePreset(pendingFinalName, newConfig);
      await refreshPresets();
      await updateSettings({ ...settings, activePreset: pendingFinalName });
      void updateConfig(newConfig);
      setPendingLink(null);
      setPendingFinalName(null);
      setShowSettings(true); // open Setup so the user can verify
    } catch (e) {
      setActionError(String(e));
      setPendingLink(null);
      setPendingFinalName(null);
    }
  }

  function handleCancelLink() {
    setPendingLink(null);
    setPendingFinalName(null);
  }

  const isStreaming = status.kind !== "idle" && status.kind !== "error";

  // Capture runs as long as a device is selected — the streaming pipeline
  // attaches and detaches independently. Re-running this effect on every
  // start/stop would tear down and rebuild the cpal stream, producing a
  // few-hundred-ms gap in the local monitor each time.
  useEffect(() => {
    if (!deviceId) return;

    let cancelled = false;
    api.startAudioPreview(deviceId).catch((e) => {
      if (!cancelled) setActionError(String(e));
    });
    return () => {
      cancelled = true;
    };
  }, [deviceId]);

  const handleStart = async () => {
    setActionError(null);
    const issueKey = validateStreamConfig(
      config ? { ...config, deviceId: deviceId ?? "" } : null,
    );
    if (issueKey) {
      setActionError(t(issueKey));
      if (issueKey !== "errors.noDevice") {
        setShowSettings(true);
      }
      return;
    }
    try {
      await api.startStream(config!);
    } catch (e) {
      setActionError(String(e));
    }
  };

  const handleStop = async () => {
    setActionError(null);
    try {
      await api.stopStream();
    } catch (e) {
      setActionError(String(e));
    }
  };

  const dismissError = async () => {
    setActionError(null);
    if (status.kind === "error") {
      try {
        await api.stopStream();
      } catch {
        // ignore
      }
    }
  };

  // Stream errors get the rich dialog (with raw ffmpeg/server output);
  // client-side validation errors keep the inline banner since they're
  // single-line and actionable.
  //
  // We listen to the raw `stream-status` event instead of deriving from the
  // `status` state — the pipeline emits Error and Reconnecting back-to-back,
  // and React 18 batches both setState calls into a single render where
  // status === "reconnecting", so a useEffect on `status` would never see
  // the "error" transition.
  const [lastStreamError, setLastStreamError] = useState<
    { message: string; details: string | null } | null
  >(null);
  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    listen<StreamStatus>("stream-status", (e) => {
      const s = e.payload;
      if (s.kind === "error") {
        setLastStreamError({ message: s.message, details: s.details ?? null });
      } else if (s.kind === "live") {
        setLastStreamError(null);
      }
    }).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
  const visibleError = actionError;
  const summaryReady = useMemo(() => configLoaded && config !== null, [configLoaded, config]);
  const previewActive = !!deviceId && !isStreaming;
  const vuActive = previewActive || status.kind === "live" || status.kind === "reconnecting";

  return (
    <main className="flex h-screen flex-col bg-zinc-950 text-zinc-100">
      <header className="flex shrink-0 items-center justify-between gap-4 bg-zinc-950 px-5 py-3.5">
        <div className="flex items-center gap-2.5">
          <div className="rounded-xl bg-rose-500 p-2 text-white shadow-md shadow-rose-500/30">
            <Radio className="h-4 w-4" />
          </div>
          <h1 className="text-base font-semibold tracking-tight">Aircast</h1>
        </div>

        <ModeSwitch value={mode} onChange={setMode} />

        <div className="flex items-center gap-2">
          <DevicePill value={deviceId} onChange={setDeviceId} />
          <button
            type="button"
            onClick={() => setShowSettings(true)}
            title={t("header.setup")}
            className="flex items-center gap-1.5 rounded-full bg-zinc-800 px-3 py-1.5 text-xs font-semibold text-zinc-200 transition-colors hover:bg-zinc-700"
          >
            <SettingsIcon className="h-3.5 w-3.5" />
            <span>{t("header.setup")}</span>
          </button>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {visibleError && (
          <div className="px-5 pt-4">
            <div className={mode === "simple" ? "mx-auto w-full max-w-2xl" : ""}>
              <ErrorBanner
                message={visibleError}
                onDismiss={dismissError}
                onOpenSetup={() => setShowSettings(true)}
              />
            </div>
          </div>
        )}

        <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-5">
          {mode === "simple" ? (
            <div className="mx-auto w-full max-w-2xl">
              <SimpleMode
                level={level}
                vuActive={vuActive}
                config={summaryReady ? config : null}
                status={status}
                onStart={handleStart}
                onStop={handleStop}
                deviceReady={!!deviceId}
              />
            </div>
          ) : (
            <StudioMode
              level={level}
              config={summaryReady ? config : null}
              status={status}
              onStart={handleStart}
              onStop={handleStop}
              deviceReady={!!deviceId}
            />
          )}
        </div>
      </div>

      <StatusBar status={status} micOpen={micOpen} deviceName={deviceId} />

      {config && (
        <SettingsModal
          open={showSettings}
          onClose={() => setShowSettings(false)}
          config={config}
          onConfigChange={(c) => void updateConfig(c)}
          settings={settings}
          onSettingsChange={(s) => void updateSettings(s)}
          onPasteLink={handlePasteLink}
        />
      )}

      <AddServerFromLinkModal
        parsed={pendingLink}
        finalName={pendingFinalName}
        onCancel={handleCancelLink}
        onConfirm={handleConfirmLink}
      />

      <ErrorDialog
        open={lastStreamError !== null}
        message={lastStreamError?.message ?? ""}
        details={lastStreamError?.details ?? null}
        onClose={async () => {
          setLastStreamError(null);
          // Dismissing the dialog also stops any ongoing reconnect loop, so
          // the user gets a clean state to retry or reconfigure.
          try {
            await api.stopStream();
          } catch {
            // ignore
          }
        }}
        onOpenSetup={() => {
          setLastStreamError(null);
          api.stopStream().catch(() => {});
          setShowSettings(true);
        }}
      />
    </main>
  );
}

function ModeSwitch({ value, onChange }: { value: AppMode; onChange: (m: AppMode) => void }) {
  const { t } = useT();
  const isStudio = value === "studio";
  return (
    <div className="relative flex items-center rounded-full bg-zinc-900 p-1 shadow-inner shadow-black/30">
      <span
        aria-hidden
        className="absolute inset-y-1 left-1 w-[calc(50%-0.25rem)] rounded-full bg-gradient-to-b from-rose-400 to-rose-500 shadow-md shadow-rose-500/30 transition-transform duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
        style={{
          transform: isStudio ? "translateX(100%)" : "translateX(0)",
        }}
      />
      <ModeButton
        active={!isStudio}
        onClick={() => onChange("simple")}
        icon={<Mic className="h-3.5 w-3.5" />}
      >
        {t("mode.simple")}
      </ModeButton>
      <ModeButton
        active={isStudio}
        onClick={() => onChange("studio")}
        icon={<SlidersHorizontal className="h-3.5 w-3.5" />}
      >
        {t("mode.studio")}
      </ModeButton>
    </div>
  );
}

function ModeButton({
  children,
  active,
  onClick,
  icon,
}: {
  children: React.ReactNode;
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "relative z-10 flex w-28 items-center justify-center gap-1.5 rounded-full px-4 py-2 text-xs font-semibold tracking-wide transition-colors duration-200",
        active ? "text-white" : "text-zinc-500 hover:text-zinc-300",
      ].join(" ")}
    >
      <span
        className={`transition-transform duration-300 ${active ? "scale-110" : "scale-100"}`}
      >
        {icon}
      </span>
      <span>{children}</span>
    </button>
  );
}
