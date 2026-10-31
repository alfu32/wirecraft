#!/usr/bin/env python3
"""Compose the Microsoft Store artwork from the captured application screenshots.

Every picture inside the artwork is an unretouched crop of a screenshot in
packaging/microsoft-store/assets (captured by capture_store_features.py); the only
text added is the product name, so one set serves every listing language. The
1920x1080 hero carries no text. Requires Pillow and the DejaVu fonts.
"""

import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "packaging/microsoft-store/assets"
ART = ASSETS / "art"
ICON = ROOT / "packaging/wirecraft.png"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
BACKGROUND = (13, 22, 32)
ACCENT = (117, 212, 214)

# (screenshot, crop box in the 1920x1080 capture): the regions the artwork shows.
CROPS = {
    "topology": ("screenshots/01-designer-topology.png", (262, 170, 1560, 700)),
    "routing": ("screenshots/02-designer-routing.png", (262, 172, 1920, 1030)),
    "component": ("screenshots/03-component-ports.png", (262, 172, 1920, 1030)),
    "completion": ("screenshots/06-editor-completion.png", (290, 250, 1400, 840)),
    "header": ("screenshots/05-editor-generated-header.png", (290, 250, 1100, 860)),
    "hover": ("screenshots/07-editor-hover.png", (262, 230, 1100, 760)),
    "export": ("screenshots/08-export-plan.png", (585, 285, 1570, 975)),
    "kanban": ("screenshots/09-project-management.png", (0, 90, 1290, 900)),
    "mcp": ("screenshots/10-mcp-server.png", (0, 90, 960, 600)),
    "analysis": ("extra/extra-analysis.png", (0, 110, 960, 900)),
}


def crop(name):
    file, box = CROPS[name]
    return Image.open(ASSETS / file).convert("RGB").crop(box)


def canvas(size):
    """The designer's dark grid, as the application draws its canvas."""
    image = Image.new("RGB", size, BACKGROUND)
    draw = ImageDraw.Draw(image)
    for x in range(0, size[0], 25):
        draw.line([(x, 0), (x, size[1])], fill=(20, 31, 43) if x % 125 else (26, 40, 54))
    for y in range(0, size[1], 25):
        draw.line([(0, y), (size[0], y)], fill=(20, 31, 43) if y % 125 else (26, 40, 54))
    return image


def place(image, picture, box, radius=14):
    """Fit `picture` into `box`, keeping its aspect by cropping the right or bottom edge
    (code and canvases read from the top left), with a frame and shadow."""
    x, y, w, h = box
    scale = max(w / picture.width, h / picture.height)
    resized = picture.resize((round(picture.width * scale), round(picture.height * scale)), Image.LANCZOS)
    resized = resized.crop((0, 0, w, h))
    shadow = Image.new("L", (w + 60, h + 60), 0)
    ImageDraw.Draw(shadow).rounded_rectangle([30, 36, w + 30, h + 36], radius, fill=150)
    image.paste((0, 0, 0), (x - 30, y - 30), shadow.filter(ImageFilter.GaussianBlur(14)))
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], radius, fill=255)
    image.paste(resized, (x, y), mask)
    ImageDraw.Draw(image).rounded_rectangle([x, y, x + w - 1, y + h - 1], radius, outline=(52, 74, 92), width=2)


def brand(image, x, y, size, name=True):
    icon = Image.open(ICON).convert("RGBA").resize((size, size), Image.LANCZOS)
    image.paste(icon, (x, y), icon)
    if name:
        font = ImageFont.truetype(FONT, round(size * 0.62))
        draw = ImageDraw.Draw(image)
        top = y + (size - font.getbbox("Wirecraft")[3]) // 2
        draw.text((x + size + round(size * 0.3), top), "Wirecraft", font=font, fill=(234, 242, 247))


def logo(size):
    icon = Image.open(ICON).convert("RGBA")
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    inner = round(size * 0.86)
    icon = icon.resize((inner, inner), Image.LANCZOS)
    image.paste(icon, ((size - inner) // 2, (size - inner) // 2), icon)
    return image


def hero():
    image = canvas((1920, 1080))
    place(image, crop("routing"), (90, 110, 1180, 640))
    place(image, crop("component"), (1150, 80, 680, 420), radius=12)
    place(image, crop("completion"), (1000, 560, 830, 446), radius=12)
    place(image, crop("kanban"), (150, 790, 760, 230), radius=12)
    return image


def square():
    image = canvas((1080, 1080))
    brand(image, 70, 60, 110)
    place(image, crop("topology"), (70, 230, 940, 420))
    place(image, crop("header"), (70, 690, 455, 320), radius=12)
    place(image, crop("mcp"), (555, 690, 455, 320), radius=12)
    return image


def poster():
    image = canvas((720, 1080))
    brand(image, 50, 54, 92)
    place(image, crop("routing"), (50, 200, 620, 300))
    place(image, crop("hover"), (50, 530, 620, 230), radius=12)
    place(image, crop("export"), (50, 790, 300, 240), radius=12)
    place(image, crop("analysis"), (370, 790, 300, 240), radius=12)
    return image


def promo_1000():
    image = canvas((1000, 800))
    brand(image, 50, 40, 80)
    place(image, crop("topology"), (50, 160, 440, 280), radius=12)
    place(image, crop("completion"), (510, 160, 440, 280), radius=12)
    place(image, crop("kanban"), (50, 470, 440, 280), radius=12)
    place(image, crop("export"), (510, 470, 440, 280), radius=12)
    return image


def promo_358():
    image = canvas((358, 358))
    brand(image, 129, 40, 100, name=False)
    font = ImageFont.truetype(FONT, 40)
    width = font.getlength("Wirecraft")
    ImageDraw.Draw(image).text(((358 - width) / 2, 160), "Wirecraft", font=font, fill=(234, 242, 247))
    place(image, crop("routing"), (24, 230, 310, 108), radius=10)
    return image


def promo_414():
    image = canvas((414, 180))
    brand(image, 24, 56, 68)
    return image


OUTPUTS = {
    "hero-1920x1080.png": hero,
    "square-1080x1080.png": square,
    "poster-720x1080.png": poster,
    "promo-1000x800.png": promo_1000,
    "promo-358x358.png": promo_358,
    "promo-414x180.png": promo_414,
    "logo-300x300.png": lambda: logo(300),
    "logo-150x150.png": lambda: logo(150),
    "logo-71x71.png": lambda: logo(71),
}


def main():
    ART.mkdir(parents=True, exist_ok=True)
    sources = {}
    for name, build in OUTPUTS.items():
        build().save(ART / name, optimize=True)
        print(ART / name)
    for file, _ in CROPS.values():
        sources[file] = hashlib.sha256((ASSETS / file).read_bytes()).hexdigest()
    (ART / "sources.json").write_text(json.dumps(dict(
        method="Pillow composition of unretouched screenshot crops; only the product name is added",
        screenshots=sources, crops={k: list(v[1]) for k, v in CROPS.items()},
    ), indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
