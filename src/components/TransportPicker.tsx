import { Check, Globe, RadioTower } from "lucide-react";
import { useT } from "@/i18n/context";
import type { Transport } from "@/types";

interface Props {
  value: Transport;
  onChange: (transport: Transport) => void;
}

/**
 * Transport chooser for the server form.
 *
 * Two cards rather than a dropdown: there are exactly two options, the choice
 * changes what every field below it means, and the reason to pick the second
 * one — "works where the streaming port is blocked" — is the whole feature.
 * A `<select>` hides all of that behind a click.
 *
 * Real radio inputs underneath, so arrow keys, labels and form semantics work;
 * the cards are just the skin.
 */
export function TransportPicker({ value, onChange }: Props) {
  const { t } = useT();

  return (
    <fieldset className="flex flex-col gap-2 border-0 p-0">
      <legend className="mb-2 text-xs text-zinc-500">
        {t("settings.transport")}
      </legend>
      <div className="grid grid-cols-2 gap-2.5">
        <Option
          transport="icecast"
          selected={value === "icecast"}
          onSelect={onChange}
          icon={<RadioTower className="h-[18px] w-[18px]" strokeWidth={1.75} />}
          title={t("settings.transportIcecast")}
          description={t("settings.transportIcecastShort")}
        />
        <Option
          transport="webcast"
          selected={value === "webcast"}
          onSelect={onChange}
          icon={<Globe className="h-[18px] w-[18px]" strokeWidth={1.75} />}
          title={t("settings.transportWebcast")}
          description={t("settings.transportWebcastShort")}
        />
      </div>
    </fieldset>
  );
}

function Option({
  transport,
  selected,
  onSelect,
  icon,
  title,
  description,
}: {
  transport: Transport;
  selected: boolean;
  onSelect: (t: Transport) => void;
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <label
      className={[
        // Concentric radius: 8px inner tile + 12px padding = 20px outer.
        "group relative flex cursor-pointer flex-col gap-2.5 rounded-[20px] p-3",
        "ring-1 ring-inset",
        // Only the properties that actually change — never `transition: all`.
        "transition-[background-color,box-shadow,scale] duration-200",
        "ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96] motion-reduce:transition-none",
        "focus-within:ring-2 focus-within:ring-rose-500/70",
        selected
          ? // Depth from layered shadow rather than a hard border.
            "bg-rose-500/10 ring-rose-500/50 shadow-[0_1px_2px_rgba(0,0,0,0.4),0_10px_28px_-12px_rgba(244,63,94,0.45)]"
          : "bg-zinc-800/40 ring-zinc-700/60 hover:bg-zinc-800/70 hover:ring-zinc-600/70",
      ].join(" ")}
    >
      <input
        type="radio"
        name="transport"
        value={transport}
        checked={selected}
        onChange={() => onSelect(transport)}
        className="sr-only"
      />

      <div className="flex items-start justify-between gap-2">
        <span
          className={[
            "flex h-9 w-9 items-center justify-center rounded-lg",
            "transition-[background-color,color] duration-200 ease-[cubic-bezier(0.2,0,0,1)]",
            "motion-reduce:transition-none",
            selected
              ? "bg-rose-500/20 text-rose-300"
              : "bg-zinc-700/50 text-zinc-400 group-hover:text-zinc-300",
          ].join(" ")}
        >
          {icon}
        </span>

        {/* Kept in the DOM and cross-faded, so it animates both ways. */}
        <span
          aria-hidden="true"
          className={[
            "mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-rose-500 text-white",
            "transition-[opacity,scale,filter] duration-200 ease-[cubic-bezier(0.2,0,0,1)]",
            "motion-reduce:transition-none",
            selected
              ? "scale-100 opacity-100 blur-0"
              : "scale-[0.25] opacity-0 blur-[4px]",
          ].join(" ")}
        >
          <Check className="h-3 w-3" strokeWidth={3} />
        </span>
      </div>

      <div className="flex flex-col gap-0.5">
        <span
          className={[
            "text-sm font-medium text-balance",
            "transition-colors duration-200 motion-reduce:transition-none",
            selected ? "text-zinc-50" : "text-zinc-300",
          ].join(" ")}
        >
          {title}
        </span>
        <span className="text-xs leading-snug text-pretty text-zinc-500">
          {description}
        </span>
      </div>
    </label>
  );
}
