import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Cable, Mic, Radio, Settings as SettingsIcon, SlidersHorizontal } from "lucide-react";
import { AboutModal } from "@/components/AboutModal";
import { AddServerFromLinkModal } from "@/components/AddServerFromLinkModal";
import { BroadcastTitleStrip } from "@/components/BroadcastTitleStrip";
import { ConfirmModeSwitchModal } from "@/components/ConfirmModeSwitchModal";
import { DevicePill } from "@/components/DevicePill";
import { ErrorBanner } from "@/components/ErrorBanner";
import { ErrorDialog } from "@/components/ErrorDialog";
import { RelayMode } from "@/components/RelayMode";
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
import { useBroadcastTitle } from "@/hooks/useBroadcastTitle";
import { useMode } from "@/hooks/useMode";
import { useMicOpen } from "@/hooks/useMicOpen";
import { useMusic } from "@/hooks/useMusic";
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
  const [settingsInitialSection, setSettingsInitialSection] = useState<
    "metadata" | "relay" | null
  >(null);
  const [showAbout, setShowAbout] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingModeSwitch, setPendingModeSwitch] = useState<AppMode | null>(null);

  const { config, update: updateConfig, loaded: configLoaded } = useCurrentConfig(deviceId);
  const { presets, refresh: refreshPresets } = usePresets();
  const { mode, setMode } = useMode();
  const { open: micOpen } = useMicOpen();
  const { snapshot: musicSnapshot } = useMusic();
  const broadcastTitle = useBroadcastTitle();
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

  // Mode change while live has audible side effects on the antenna (music
  // stops, mic gating flips). Intercept the switch and surface a modal that
  // explains the consequences before applying it. Off-air switches go
  // straight through.
  //
  // Note: we read `isStreaming` and `mode` from refs rather than via
  // `useCallback` deps. Vite's Fast Refresh has a known failure mode where
  // a memoized callback keeps a stale closure of `isStreaming` after a hot
  // reload, which made the warning silently skip during development. Refs
  // are always up-to-date with no memo-cache to invalidate.
  const isStreamingRef = useRef(isStreaming);
  isStreamingRef.current = isStreaming;
  const modeRef = useRef(mode);
  modeRef.current = mode;

  const requestModeChange = useCallback(
    (next: AppMode) => {
      if (next === modeRef.current) return;
      if (isStreamingRef.current) {
        setPendingModeSwitch(next);
      } else {
        void setMode(next);
      }
    },
    [setMode],
  );

  const confirmModeSwitch = useCallback(() => {
    if (pendingModeSwitch) {
      void setMode(pendingModeSwitch);
    }
    setPendingModeSwitch(null);
  }, [pendingModeSwitch, setMode]);

  const cancelModeSwitch = useCallback(() => {
    setPendingModeSwitch(null);
  }, []);

  // Audio input lifecycle:
  // - Simple/Studio: cpal mic capture for the selected device.
  // - Relay: ffmpeg URL decoder for the active relay source.
  // Starting one tears down the other on the backend, so we can freely
  // re-run this effect when the user switches mode or input.
  useEffect(() => {
    let cancelled = false;
    if (mode === "relay") {
      const name = settings.activeRelaySource;
      if (!name) return;
      api.startRelayInput(name).catch((e) => {
        if (!cancelled) setActionError(String(e));
      });
    } else if (deviceId) {
      api.startAudioPreview(deviceId).catch((e) => {
        if (!cancelled) setActionError(String(e));
      });
    }
    return () => {
      cancelled = true;
    };
  }, [mode, deviceId, settings.activeRelaySource]);

  const handleStart = async () => {
    setActionError(null);
    // Relay mode has no mic device — the upstream URL plays that role. We
    // still validate everything else, just bypass the device check by
    // pretending one is set.
    const effectiveDeviceId = mode === "relay" ? "relay" : deviceId ?? "";
    const issueKey = validateStreamConfig(
      config ? { ...config, deviceId: effectiveDeviceId } : null,
    );
    if (issueKey) {
      setActionError(t(issueKey));
      if (issueKey !== "errors.noDevice") {
        setShowSettings(true);
      }
      return;
    }
    if (mode === "relay" && !settings.activeRelaySource) {
      setActionError(t("errors.noRelaySource"));
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
      <header className="grid shrink-0 grid-cols-3 items-center gap-4 bg-zinc-950 px-5 py-3.5">
        <div className="flex items-center gap-2.5 justify-self-start">
          <div className="rounded-xl bg-rose-500 p-2 text-white shadow-md shadow-rose-500/30">
            <Radio className="h-4 w-4" />
          </div>
          <h1 className="text-base font-semibold tracking-tight">Aircast</h1>
        </div>

        <div className="justify-self-center">
          <ModeSwitch
            value={mode}
            onChange={requestModeChange}
            enabled={settings.enabledModes}
          />
        </div>

        <div className="flex items-center gap-2 justify-self-end">
          {/* In Studio mode the device selector lives inside the MicPanel
              (next to the mic-open toggle) so the user has one place that
              owns the microphone control. The header pill is therefore only
              shown in Simple mode. */}
          {mode === "simple" && <DevicePill value={deviceId} onChange={setDeviceId} />}
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
          {mode === "simple" && (
            <div className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center">
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
          )}
          {mode === "studio" && (
            <StudioMode
              level={level}
              config={summaryReady ? config : null}
              status={status}
              onStart={handleStart}
              onStop={handleStop}
              deviceReady={!!deviceId}
              deviceId={deviceId}
              onDeviceChange={setDeviceId}
              broadcastTitle={broadcastTitle}
              onEditBroadcast={() => {
                setSettingsInitialSection("metadata");
                setShowSettings(true);
              }}
            />
          )}
          {mode === "relay" && (
            <div className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center">
              <RelayMode
                level={level}
                vuActive={vuActive}
                config={summaryReady ? config : null}
                status={status}
                onStart={handleStart}
                onStop={handleStop}
                onOpenRelaySources={() => {
                  setSettingsInitialSection("relay");
                  setShowSettings(true);
                }}
                activeSourceName={settings.activeRelaySource ?? null}
                sources={settings.relaySources}
                onPickSource={(name) => {
                  void api.setActiveRelaySource(name);
                  void api.startRelayInput(name);
                  void updateSettings({ ...settings, activeRelaySource: name });
                }}
              />
            </div>
          )}
        </div>
      </div>

      {/* Strip is Simple-mode only; Studio mode shows the same info as a chip
          inside the NowPlaying card to avoid duplicating "À l'antenne" with
          the StatusBar. */}
      {mode === "simple" && (
        <BroadcastTitleStrip
          title={broadcastTitle}
          live={status.kind === "live"}
          onEdit={() => {
            setSettingsInitialSection("metadata");
            setShowSettings(true);
          }}
        />
      )}

      <StatusBar
        status={status}
        micOpen={micOpen}
        deviceName={deviceId}
        onAboutClick={() => setShowAbout(true)}
        mode={mode}
      />

      {config && (
        <SettingsModal
          open={showSettings}
          onClose={() => {
            setShowSettings(false);
            setSettingsInitialSection(null);
          }}
          config={config}
          onConfigChange={(c) => void updateConfig(c)}
          settings={settings}
          onSettingsChange={(s) => void updateSettings(s)}
          onPasteLink={handlePasteLink}
          initialSection={settingsInitialSection}
        />
      )}

      <AddServerFromLinkModal
        parsed={pendingLink}
        finalName={pendingFinalName}
        onCancel={handleCancelLink}
        onConfirm={handleConfirmLink}
      />

      <AboutModal open={showAbout} onClose={() => setShowAbout(false)} />

      <ConfirmModeSwitchModal
        source={mode}
        target={pendingModeSwitch}
        musicPlaying={musicSnapshot.state === "playing"}
        micOpen={micOpen}
        onCancel={cancelModeSwitch}
        onConfirm={confirmModeSwitch}
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

function ModeSwitch({
  value,
  onChange,
  enabled,
}: {
  value: AppMode;
  onChange: (m: AppMode) => void;
  enabled: { simple: boolean; studio: boolean; relay: boolean };
}) {
  const { t } = useT();
  // Compose the list of visible modes in stable order. If only one mode is
  // visible, hide the switch entirely — there's nothing to switch.
  // Studio first as it's the most used mode in production. Simple is the
  // entry-level fallback, Relay the niche use-case.
  const visible: AppMode[] = (["studio", "simple", "relay"] as AppMode[]).filter(
    (m) => enabled[m],
  );
  if (visible.length < 2) return null;

  const idx = Math.max(0, visible.indexOf(value));

  // Each ModeButton has a fixed w-28 (7rem) width, so the sliding indicator
  // can be sized to match and translated by full button widths (`100%` =
  // its own width = one button).
  return (
    <div className="relative flex items-center rounded-full bg-zinc-900 p-1 shadow-inner shadow-black/30">
      <span
        aria-hidden
        className="absolute inset-y-1 left-1 w-28 rounded-full bg-gradient-to-b from-rose-400 to-rose-500 shadow-md shadow-rose-500/30 transition-transform duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
        style={{ transform: `translateX(${idx * 100}%)` }}
      />
      {visible.map((m) => (
        <ModeButton
          key={m}
          active={m === value}
          onClick={() => onChange(m)}
          icon={modeIcon(m)}
        >
          {t(`mode.${m}`)}
        </ModeButton>
      ))}
    </div>
  );
}

function modeIcon(m: AppMode) {
  switch (m) {
    case "simple":
      return <Mic className="h-3.5 w-3.5" />;
    case "studio":
      return <SlidersHorizontal className="h-3.5 w-3.5" />;
    case "relay":
      return <Cable className="h-3.5 w-3.5" />;
  }
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
