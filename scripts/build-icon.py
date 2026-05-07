#!/usr/bin/env python3
"""Generate the Aircast source icon (1024×1024 PNG).

Run from the repo root:
    python3 scripts/build-icon.py

Then rasterise platform-specific variants with:
    pnpm tauri icon src-tauri/icons/icon.png
"""

from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons" / "icon.png"

SIZE = 1024
BG = "#F43F5E"  # tailwind rose-500
FG = "#FFFFFF"
RADIUS = 225  # Apple-style squircle (~22% of edge)

# Symmetric audio-waveform glyph: 7 rounded vertical bars whose heights step
# up and back down. Reads as "audio" at any size and stays balanced.
BAR_W = 88
BAR_GAP = 56
BAR_HEIGHTS = [220, 380, 540, 660, 540, 380, 220]


def main() -> None:
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    draw.rounded_rectangle((0, 0, SIZE, SIZE), radius=RADIUS, fill=BG)

    n = len(BAR_HEIGHTS)
    total_w = n * BAR_W + (n - 1) * BAR_GAP
    start_x = (SIZE - total_w) // 2
    cy = SIZE // 2

    for i, h in enumerate(BAR_HEIGHTS):
        x0 = start_x + i * (BAR_W + BAR_GAP)
        y0 = cy - h // 2
        x1 = x0 + BAR_W
        y1 = y0 + h
        draw.rounded_rectangle((x0, y0, x1, y1), radius=BAR_W // 2, fill=FG)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    img.save(OUT, "PNG")
    print(f"wrote {OUT.relative_to(ROOT)} ({SIZE}×{SIZE})")


if __name__ == "__main__":
    main()
