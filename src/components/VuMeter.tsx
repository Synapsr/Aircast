import { useEffect, useRef, useState } from "react";

interface Props {
  level: number;
  active: boolean;
}

const PEAK_DECAY_PER_FRAME = 0.012;

export function VuMeter({ level, active }: Props) {
  const [peak, setPeak] = useState(0);
  const peakRef = useRef(0);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    if (level > peakRef.current) {
      peakRef.current = level;
      setPeak(level);
    }
  }, [level]);

  useEffect(() => {
    let last = performance.now();
    const tick = (now: number) => {
      const dt = (now - last) / (1000 / 60); // normalize to 60fps frames
      last = now;
      const decay = PEAK_DECAY_PER_FRAME * dt;
      const next = Math.max(0, peakRef.current - decay);
      if (next !== peakRef.current) {
        peakRef.current = next;
        setPeak(next);
      }
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  const displayLevel = active ? level : 0;
  const displayPeak = active ? peak : 0;
  const pct = clamp(displayLevel * 100);
  const peakPct = clamp(displayPeak * 100);

  return (
    <div className="flex items-center gap-3">
      <div
        className="relative h-2.5 w-full overflow-hidden rounded-full"
        style={{
          background:
            "linear-gradient(90deg, #10b981 0%, #10b981 55%, #f59e0b 78%, #ef4444 100%)",
        }}
      >
        {/* Cover panel from the right "eats" the unfilled portion. The gradient
            beneath stays at full container width, so green/yellow/red sit at
            fixed positions instead of being compressed into the level bar. */}
        <div
          className="absolute inset-y-0 right-0 bg-zinc-800 transition-[width] duration-75 ease-out"
          style={{ width: `${100 - pct}%` }}
        />
        <div
          className="absolute inset-y-0 w-0.5 bg-white/90 transition-opacity"
          style={{ left: `calc(${peakPct}% - 1px)`, opacity: active && peakPct > 0 ? 0.9 : 0 }}
        />
        {/* 0dB tick marks */}
        <div className="pointer-events-none absolute inset-y-0 left-[55%] w-px bg-white/10" />
        <div className="pointer-events-none absolute inset-y-0 left-[78%] w-px bg-white/10" />
      </div>
      <span className="w-10 shrink-0 text-right font-mono text-[10px] tabular-nums text-zinc-500">
        {Math.round(pct)}%
      </span>
    </div>
  );
}

function clamp(n: number): number {
  return Math.min(100, Math.max(0, n));
}
