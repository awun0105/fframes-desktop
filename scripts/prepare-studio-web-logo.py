"""Create small, transparent website logos from the approved PNG master.

Run with Pillow installed: python scripts/prepare-studio-web-logo.py
The master and desktop icon are preserved. Encoded assets are committed so the
deployment build needs only Node, not Python or an image processing dependency.
"""

from pathlib import Path

from PIL import Image


def main():
    brand = Path(__file__).resolve().parents[1] / "landing/studio/brand"
    source = brand / "fframes-studio-logo-blue-v1.png"
    with Image.open(source) as master:
        image = master.convert("RGBA")
    # Ignore almost invisible stray pixels when measuring the useful artwork.
    left, top, right, bottom = image.getchannel("A").point(
        lambda value: 255 if value >= 8 else 0
    ).getbbox()
    image = image.crop((left - 2, top - 2, right + 2, bottom + 2))
    for width in (320, 640):
        size = (width, round(image.height * width / image.width))
        logo = image.resize(size, Image.Resampling.LANCZOS)
        # Sixteen alpha levels retain smooth edges at this display size while
        # compressing the generated master's noisy, near-transparent pixels.
        logo.putalpha(logo.getchannel("A").point(
            lambda value: round(value / 255 * 15) * 17
        ))
        output = brand / f"studio-logo-{width}.webp"
        logo.save(output, "WEBP", quality=82, method=6)
        print(f"{output.name}: {size[0]}x{size[1]}, {output.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
