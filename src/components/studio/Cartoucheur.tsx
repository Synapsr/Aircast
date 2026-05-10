import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { useT } from "@/i18n/context";
import type { CartSnapshot } from "@/types";

// 3×3 grid keeps the right column shorter so it lines up with the music
// queue on the left rather than overflowing past it. Backend persists slots
// by index so existing slots 10-12 in user data simply stay hidden until a
// future release brings them back.
const CART_COUNT = 9;

interface Props {
  carts: CartSnapshot[];
  onChange: () => void;
}

export function Cartoucheur({ carts, onChange }: Props) {
  const { t } = useT();
  const cartBySlot = new Map(carts.map((c) => [c.slot, c]));

  return (
    <section className="flex h-full flex-col rounded-2xl bg-zinc-900">
      <header className="flex items-center justify-between px-5 py-4">
        <h3 className="text-sm font-medium text-zinc-200">{t("carts.title")}</h3>
        <span className="text-xs text-zinc-500">{t("carts.hint")}</span>
      </header>

      <div className="grid grid-cols-3 gap-2 px-3 pb-3">
        {Array.from({ length: CART_COUNT }, (_, i) => {
          const slot = i + 1;
          const cart = cartBySlot.get(slot);
          return <CartButton key={slot} slot={slot} cart={cart} onChange={onChange} />;
        })}
      </div>
    </section>
  );
}

function CartButton({
  slot,
  cart,
  onChange,
}: {
  slot: number;
  cart: CartSnapshot | undefined;
  onChange: () => void;
}) {
  const { t } = useT();
  async function handleAssign() {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["mp3", "wav", "flac", "ogg", "m4a"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    const path = selected;
    const filename = path.split("/").pop()?.split("\\").pop() ?? "Cart";
    const name = filename.replace(/\.[^.]+$/, "");
    await api.cartAssign(slot, name, path);
    onChange();
  }

  async function handleClick() {
    if (!cart) {
      void handleAssign();
      return;
    }
    if (cart.playing) {
      await api.cartStop(slot);
    } else {
      await api.cartPlay(slot);
    }
    onChange();
  }

  async function handleRemove(e: React.MouseEvent) {
    e.stopPropagation();
    await api.cartRemove(slot);
    onChange();
  }

  if (!cart) {
    return (
      <button
        type="button"
        onClick={handleClick}
        className="group flex aspect-[4/3] flex-col items-center justify-center gap-1.5 rounded-xl bg-zinc-800/50 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-300"
      >
        <Plus className="h-5 w-5" />
        <span className="text-[10px] font-medium uppercase tracking-wider">
          {t("carts.slot", { n: slot })}
        </span>
      </button>
    );
  }

  const progress =
    cart.durationSecs > 0 ? Math.min(100, (cart.elapsedSecs / cart.durationSecs) * 100) : 0;

  return (
    <button
      type="button"
      onClick={handleClick}
      className={[
        "group relative flex aspect-[4/3] flex-col items-center justify-center gap-1 overflow-hidden rounded-xl p-2 text-center transition-all",
        cart.playing
          ? "bg-rose-500 text-white shadow-lg shadow-rose-500/30"
          : "bg-zinc-800 text-zinc-100 hover:bg-zinc-700",
      ].join(" ")}
    >
      <span
        className={`absolute right-1.5 top-1.5 text-[10px] font-mono ${cart.playing ? "text-rose-200" : "text-zinc-500"}`}
      >
        {String(slot).padStart(2, "0")}
      </span>
      <span className="line-clamp-2 max-w-full text-xs font-semibold leading-tight">
        {cart.name}
      </span>
      <span
        className={`text-[11px] tabular-nums ${cart.playing ? "text-rose-100" : "text-zinc-500"}`}
      >
        {formatTime(cart.playing ? cart.elapsedSecs : cart.durationSecs)}
      </span>
      {cart.playing && (
        <div className="absolute inset-x-0 bottom-0 h-1 bg-rose-700/40">
          <div
            className="h-full bg-white/80 transition-[width] duration-100"
            style={{ width: `${progress}%` }}
          />
        </div>
      )}
      <button
        type="button"
        onClick={handleRemove}
        title={t("carts.remove")}
        className={[
          "absolute left-1.5 top-1.5 hidden rounded-full p-1 transition-colors group-hover:block",
          cart.playing
            ? "text-rose-100 hover:bg-rose-600"
            : "text-zinc-500 hover:bg-zinc-700 hover:text-rose-300",
        ].join(" ")}
      >
        <Trash2 className="h-3 w-3" />
      </button>
    </button>
  );
}

function formatTime(seconds: number): string {
  const total = Math.floor(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
