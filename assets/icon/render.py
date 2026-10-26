#!/usr/bin/env python3
"""Render the raster app icons from archipel.svg.

Requires cairosvg and Pillow (`pip install cairosvg pillow`). The outputs are
committed so builds never need these tools:

- archipel.png       512x512, window icon and Linux hicolor icon
- archipel-256.png   256x256, .deb / tarball icon
- archipel.ico       Windows (MSI, Start Menu, Add/Remove Programs)
- archipel.icns      macOS app bundle
"""

import io
from pathlib import Path

import cairosvg
from PIL import Image

HERE = Path(__file__).resolve().parent
SVG = HERE / "archipel.svg"


def render(size: int) -> Image.Image:
    png = cairosvg.svg2png(url=str(SVG), output_width=size, output_height=size)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def main() -> None:
    master = render(1024)
    render(512).save(HERE / "archipel.png", optimize=True)
    render(256).save(HERE / "archipel-256.png", optimize=True)
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    render(256).save(
        HERE / "archipel.ico",
        sizes=[(s, s) for s in ico_sizes],
        append_images=[render(s) for s in ico_sizes],
    )
    master.save(HERE / "archipel.icns")
    for name in ["archipel.png", "archipel-256.png", "archipel.ico", "archipel.icns"]:
        print(f"wrote {name} ({(HERE / name).stat().st_size} bytes)")


if __name__ == "__main__":
    main()
