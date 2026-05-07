#!/usr/bin/env python3
"""Generate the Aircast source icon (1024×1024 PNG).

Run from the repo root:
    python3 scripts/build-icon.py

Then rasterise platform-specific variants with:
    pnpm tauri icon src-tauri/icons/icon.png

The glyph is a faithful 1024×1024 reproduction of the lucide `Radio` icon
that the app already uses in the header — a centre dot with two pairs of
arcs broadcasting outward to the left and right. We render at 4× and
downsample with LANCZOS for smooth edges.
"""

from math import cos, radians, sin
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons" / "icon.png"

SIZE = 1024
SUPER = 4  # render at 4× then downsample with LANCZOS for clean edges
W = SIZE * SUPER

BG = "#F43F5E"  # tailwind rose-500
FG = "#FFFFFF"
RADIUS = 225 * SUPER  # Apple-style squircle (~22% of edge)

# lucide Radio geometry, scaled from a 24-unit viewBox.
CX, CY = W // 2, W // 2
DOT_R = 60 * SUPER
INNER_R = 175 * SUPER  # centreline radius of the inner pair of arcs
OUTER_R = 290 * SUPER  # centreline radius of the outer pair of arcs
STROKE = 60 * SUPER

# Each arc spans 90°: ±45° around the horizontal axis. PIL angles: 0 = right,
# 90 = down, clockwise (because the y axis points down).
RIGHT = (-45.0, 45.0)
LEFT = (135.0, 225.0)


def stroked_band_points(R_centre: float, start_deg: float, end_deg: float):
    """Return the polygon points outlining a stroked arc with rounded caps.

    The polygon is: outer arc → end cap (semicircle bulging outward of the
    arc) → inner arc (reverse) → start cap (semicircle bulging outward).
    Filling this with a single `polygon()` call avoids any visible seam at
    the junction between the arc body and the round caps.
    """
    half = STROKE / 2.0
    r_outer = R_centre + half
    r_inner = R_centre - half
    r_cap = half

    n_arc = 96
    n_cap = 24

    pts = []

    # Outer arc, start → end
    for i in range(n_arc + 1):
        t = start_deg + (end_deg - start_deg) * i / n_arc
        pts.append((CX + r_outer * cos(radians(t)), CY + r_outer * sin(radians(t))))

    # End cap: semicircle centred on the band centreline at end_deg, sweeping
    # from outer (cap-angle = end_deg) to inner (cap-angle = end_deg + 180),
    # bulging in the forward tangent direction (cap-angle = end_deg + 90).
    cx_e = CX + R_centre * cos(radians(end_deg))
    cy_e = CY + R_centre * sin(radians(end_deg))
    for i in range(1, n_cap):
        t = end_deg + 180.0 * i / n_cap
        pts.append((cx_e + r_cap * cos(radians(t)), cy_e + r_cap * sin(radians(t))))

    # Inner arc, end → start
    for i in range(n_arc + 1):
        t = end_deg + (start_deg - end_deg) * i / n_arc
        pts.append((CX + r_inner * cos(radians(t)), CY + r_inner * sin(radians(t))))

    # Start cap: semicircle bulging in the backward tangent direction.
    cx_s = CX + R_centre * cos(radians(start_deg))
    cy_s = CY + R_centre * sin(radians(start_deg))
    for i in range(1, n_cap):
        t = start_deg + 180.0 + 180.0 * i / n_cap
        pts.append((cx_s + r_cap * cos(radians(t)), cy_s + r_cap * sin(radians(t))))

    return pts


def main() -> None:
    big = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    draw = ImageDraw.Draw(big)

    draw.rounded_rectangle((0, 0, W, W), radius=RADIUS, fill=BG)

    for r_centre in (INNER_R, OUTER_R):
        draw.polygon(stroked_band_points(r_centre, *RIGHT), fill=FG)
        draw.polygon(stroked_band_points(r_centre, *LEFT), fill=FG)

    draw.ellipse((CX - DOT_R, CY - DOT_R, CX + DOT_R, CY + DOT_R), fill=FG)

    img = big.resize((SIZE, SIZE), Image.LANCZOS)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    img.save(OUT, "PNG")
    print(f"wrote {OUT.relative_to(ROOT)} ({SIZE}×{SIZE}, rendered at {W}×{W})")


if __name__ == "__main__":
    main()
