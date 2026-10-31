#!/usr/bin/env python3
"""Validate and build Partner Center MSIX listing import folders (stdlib only)."""

import argparse
import csv
import hashlib
import html
import io
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "packaging/microsoft-store"
EXPORT = ROOT / "packaging/listingData-9NDMQVXLGBQJ-1152921505702281824.csv"
BUNDLE = "wirecraft-store"
SCREENSHOTS = 10
# The export has a neutral English column and no en-us.
ENGLISH = "en"
ART = {
    "StoreLogo720x1080": ("poster-720x1080.png", (720, 1080)),
    "StoreLogo1080x1080": ("square-1080x1080.png", (1080, 1080)),
    "StoreLogo300x300": ("logo-300x300.png", (300, 300)),
    "StoreLogoOverride150x150": ("logo-150x150.png", (150, 150)),
    "StoreLogoOverride71x71": ("logo-71x71.png", (71, 71)),
    "PromoImage1920x1080": ("hero-1920x1080.png", (1920, 1080)),
    "OptionalPromo358x358": ("promo-358x358.png", (358, 358)),
    "OptionalPromo1000x800": ("promo-1000x800.png", (1000, 800)),
    "OptionalPromo414x180": ("promo-414x180.png", (414, 180)),
}
GROUPS = {
    "Feature": ("features", 10),
    "SearchTerm": ("searchTerms", 7),
    "DesktopScreenshotCaption": ("captions", SCREENSHOTS),
    "MinimumHardwareReq": ("minimumHardware", 2),
    "RecommendedHardwareReq": ("recommendedHardware", 2),
}
SHARED = {
    "ShortTitle": "Wirecraft",
    "SortTitle": "Wirecraft",
    "VoiceTitle": "Wirecraft",
    "CopyrightTrademarkInformation": "Copyright (c) 2026 Wirecraft contributors.",
    "AdditionalLicenseTerms": (
        "Licensed under your choice of the MIT License or Apache License 2.0 "
        "(MIT OR Apache-2.0). License texts and third-party notices accompany the application."
    ),
    "OverrideLogosForWin10": "True",
}


def store_identity(source):
    """The reserved product name and publisher shown in the listing."""
    fields = json.loads((source / "submission-fields.json").read_text(encoding="utf-8"))
    return dict(displayName=fields["productName"], publisherDisplayName=fields["publisherDisplayName"])


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_csv(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        return list(csv.reader(stream))


def encode_csv(rows):
    stream = io.StringIO(newline="")
    csv.writer(stream, lineterminator="\r\n").writerows(rows)
    return stream.getvalue().encode("utf-8-sig")


def length(text):
    # Partner Center/.NET limits must also cover supplementary Unicode characters.
    return len(text.encode("utf-16-le")) // 2


def validate_locale(locale, data):
    for key, limit in [("description", 10000), ("shortDescription", 1000)]:
        value = data.get(key)
        require(isinstance(value, str) and 0 < length(value) <= limit,
                f"{locale}: missing/oversized {key}")
        require(not re.search(r"https?://|<[^>]+>", value),
                f"{locale}: {key} must be plain listing text without URLs")
    require(isinstance(data.get("language"), str) and data["language"],
            f"{locale}: missing language label")
    for _, (key, count) in GROUPS.items():
        values = data.get(key)
        require(isinstance(values, list) and len(values) == count,
                f"{locale}: expected {count} {key}")
        for value in values:
            limit = 30 if key == "searchTerms" else 200
            require(isinstance(value, str) and 0 < length(value) <= limit,
                    f"{locale}: missing/oversized {key}: {value!r}")
            require(not value.startswith(("•", "- ", "* ")) and "\n" not in value,
                    f"{locale}: {key} must be individual plain-text entries")
    require(sum(len(term.split()) for term in data["searchTerms"]) <= 21,
            f"{locale}: search terms exceed 21 words")


def png_size(path):
    with path.open("rb") as stream:
        header = stream.read(33)
    require(header[:8] == b"\x89PNG\r\n\x1a\n" and header[12:16] == b"IHDR"
            and len(header) == 33, f"Not a PNG: {path}")
    require(path.stat().st_size <= 50 * 1024 * 1024, f"Image exceeds 50 MiB: {path}")
    return struct.unpack(">II", header[16:24])


def safe_image_path(value):
    path = PurePosixPath(value)
    require(not path.is_absolute() and ".." not in path.parts and "\\" not in value
            and len(path.parts) >= 4 and path.parts[:2] == (BUNDLE, "images")
            and path.suffix.lower() == ".png", f"Unsafe/invalid import image path: {value}")
    return Path(*path.parts[2:])


def load_source(source=SOURCE):
    template = read_csv(source / "template.csv")
    require(template[0][:4] == ["Field", "ID", "Type (Type)", "default"],
            "Unexpected Partner Center metadata header")
    locales = template[0][4:]
    require(len(set(locales)) == len(locales) and ENGLISH in locales,
            "Duplicate locale or missing English")
    files = {path.stem for path in (source / "locales").glob("*.json")}
    require(files == set(locales), "Locale files do not match the Partner Center export")
    translations = {}
    for locale in locales:
        data = json.loads((source / "locales" / f"{locale}.json").read_text(encoding="utf-8"))
        validate_locale(locale, data)
        translations[locale] = data
    manifest = json.loads((source / "assets-manifest.json").read_text(encoding="utf-8"))
    shots = manifest["screenshots"]
    require(len(shots) == SCREENSHOTS, f"Expected {SCREENSHOTS} desktop screenshots")
    for shot in shots:
        source_project = (ROOT / shot["source"]).resolve()
        require(source_project.is_relative_to(source.resolve()) and source_project.is_file(),
                f"Missing durable source project: {source_project}")
        path = source / "assets/screenshots" / shot["file"]
        require(Path(shot["file"]).name == shot["file"], "Invalid screenshot filename")
        width, height = png_size(path)
        require(width >= 1366 and height >= 768, f"Screenshot too small: {path}")
        require(shot["sha256"] == hashlib.sha256(path.read_bytes()).hexdigest(),
                f"Screenshot changed without updating provenance: {path}")
    for name, size in ART.values():
        require(png_size(source / "assets/art" / name) == size, f"Wrong dimensions: {name}")
    return template, translations, manifest


def make_rows(template, translations, manifest, locales, identity):
    require(set(locales) <= set(template[0][4:]), "Unrecognized listing locale")
    output = [template[0][:4] + locales]
    for original in template[1:]:
        if not original or not any(original):
            output.append([""] * len(output[0]))
            continue
        field = original[0]
        shared = dict(SHARED, Title=identity["displayName"],
                      DevStudio=identity["publisherDisplayName"])
        values = []
        for locale in locales:
            data = translations[locale]
            value = shared.get(field, "")
            if field == "Description":
                value = data["description"]
            elif field == "ShortDescription":
                value = data["shortDescription"]
            elif field in ART:
                value = f"{BUNDLE}/images/art/{ART[field][0]}"
            else:
                for prefix, (key, count) in GROUPS.items():
                    match = re.fullmatch(re.escape(prefix) + r"(\d+)", field)
                    if match and 1 <= int(match[1]) <= count:
                        value = data[key][int(match[1]) - 1]
                        break
                match = re.fullmatch(r"DesktopScreenshot(\d+)", field)
                if match and 1 <= int(match[1]) <= len(manifest["screenshots"]):
                    name = manifest["screenshots"][int(match[1]) - 1]["file"]
                    value = f"{BUNDLE}/images/screenshots/{name}"
            values.append(value)
        # Use English as the fallback. Every declared locale is explicitly filled.
        default = values[locales.index(ENGLISH)] if ENGLISH in locales else translations[ENGLISH].get(field, values[0])
        output.append(original[:3] + [default] + values)
    return output


def validate_rows(rows, template, images):
    require(rows[0][:4] == template[0][:4], "Metadata columns changed")
    require(len(rows) == len(template), "Partner Center row count changed")
    locales = rows[0][4:]
    require(locales and len(set(locales)) == len(locales)
            and all(locale in template[0][4:] for locale in locales), "Invalid locale header")
    for row, original in zip(rows[1:], template[1:]):
        require(len(row) == len(rows[0]), "Malformed CSV row")
        if not original or not any(original):
            require(not any(row), "Trailing blank row modified")
            continue
        require(row[:3] == original[:3], f"Field/ID/Type changed: {original[:3]}")
        if original[2].startswith("Relative path"):
            for value in row[3:]:
                if value:
                    path = images / safe_image_path(value)
                    require(path.is_file(), f"Missing image: {value}")
                    png_size(path)
    by_field = {row[0]: row for row in rows[1:] if row[0]}
    for required in ["Description", "Title", "ShortDescription", "StoreLogo300x300",
                     "DesktopScreenshot1", "Feature1", "SearchTerm1"]:
        require(all(by_field[required][3:]), f"Missing required field: {required}")


def write_review(source, translations, manifest):
    english = translations[ENGLISH]
    md = ["# Wirecraft — English Store listing", "", english["shortDescription"],
          "", "## Description", "", english["description"], "", "## Features", ""]
    md.extend("- " + feature for feature in english["features"])
    md += ["", "## Search terms", "", ", ".join(english["searchTerms"]), "",
           "## Screenshots", ""]
    for shot, caption in zip(manifest["screenshots"], english["captions"]):
        md += [f"![{caption}](assets/screenshots/{shot['file']})", "", caption, ""]
    (source / "english.md").write_text("\n".join(md), encoding="utf-8")
    # Embedded JSON works from file:// without fetch, a web server or network.
    data = json.dumps(translations, ensure_ascii=False).replace("<", "\\u003c")
    files = json.dumps([shot["file"] for shot in manifest["screenshots"]])
    options = "".join(f'<option value="{html.escape(key)}">{html.escape(value["language"])} ({key})</option>'
                      for key, value in translations.items())
    page = '''<!doctype html><html lang="en"><meta charset="utf-8">
<meta name="viewport" content="width=device-width"><title>Wirecraft Store listing review</title>
<style>body{margin:0;background:#101c29;color:#eef5fc;font:17px/1.6 system-ui,sans-serif}
main{max-width:1100px;margin:auto;padding:32px}h1{font-size:34px;line-height:1.2}
select{font:inherit;padding:8px;max-width:100%}p{white-space:pre-line}img{width:100%;height:auto;border-radius:6px}
article{margin:32px 0}.notice{color:#b6c9db}a{color:#7bd6ff}.art{display:grid;grid-template-columns:1fr 1fr;gap:20px}
.art img{object-fit:contain;max-height:400px}li{margin:8px 0}</style>
<main><h1>Wirecraft</h1><p class="notice">Prepared listing, not a live Store page.
The app interface is English. Translations have not had native-speaker review.</p>
<label for="language">Listing language </label><select id="language">OPTIONS</select>
<section id="copy"><h2 id="short"></h2><p id="description"></p><h2>Features</h2><ul id="features"></ul>
<h2>Search terms</h2><p id="terms"></p><h2>Hardware</h2><ul id="hardware"></ul><div id="gallery"></div></section>
<h2>Store artwork</h2><p class="notice">Promotional artwork, separate from screenshots.</p>
<div class="art"><img src="assets/art/hero-1920x1080.png" alt="Text-free hero art: routed topology, component ports, completion and Kanban">
<img src="assets/art/square-1080x1080.png" alt="Topology, generated header and MCP server">
<img src="assets/art/poster-720x1080.png" alt="Routing, Rust hover, plan export and network analysis">
<img src="assets/art/promo-1000x800.png" alt="Designer, completion, Kanban board and plan export">
<img src="assets/art/promo-358x358.png" alt="Wirecraft logo over routed links">
<img src="assets/art/promo-414x180.png" alt="Wirecraft logo"></div>
<p><a href="README.md">Import instructions and submission fields</a></p></main>
<script id="data" type="application/json">DATA</script><script>
const data=JSON.parse(document.getElementById('data').textContent),files=FILES;
const el=id=>document.getElementById(id);
function list(id,items){el(id).replaceChildren(...items.map(text=>{const li=document.createElement('li');li.textContent=text;return li}));}
function render(){const locale=el('language').value,d=data[locale];el('copy').dir=['ar','he'].includes(locale)?'rtl':'ltr';el('copy').lang=locale;
el('short').textContent=d.shortDescription;el('description').textContent=d.description;
list('features',d.features);el('terms').textContent=d.searchTerms.join(' · ');list('hardware',[...d.minimumHardware,...d.recommendedHardware]);
el('gallery').replaceChildren(...files.map((file,i)=>{const article=document.createElement('article'),image=document.createElement('img'),caption=document.createElement('p');
image.src='assets/screenshots/'+file;image.alt=d.captions[i];image.loading='lazy';caption.textContent=d.captions[i];article.append(image,caption);return article}));}
el('language').value='en';el('language').addEventListener('change',render);render();</script></html>'''
    page = page.replace("OPTIONS", options).replace("DATA", data).replace("FILES", files)
    (source / "review.html").write_text(page, encoding="utf-8")


def build(destination, source=SOURCE, update_export=False):
    template, translations, manifest = load_source(source)
    identity = store_identity(source)
    write_review(source, translations, manifest)
    all_rows = None
    checksums = []
    for variant, locales in [("english", [ENGLISH]),
                             ("all-languages", template[0][4:])]:
        folder = destination / variant / BUNDLE
        # Only clean this script's generated import root, never a caller's directory.
        if folder.exists():
            shutil.rmtree(folder)
        folder.mkdir(parents=True)
        for part in ["screenshots", "art"]:
            image_folder = folder / "images" / part
            image_folder.mkdir(parents=True)
            for path in (source / "assets" / part).glob("*.png"):
                shutil.copyfile(path, image_folder / path.name)
        rows = make_rows(template, translations, manifest, locales, identity)
        validate_rows(rows, template, folder / "images")
        encoded = encode_csv(rows)
        (folder / "listing.csv").write_bytes(encoded)
        require(read_csv(folder / "listing.csv") == rows, "Unicode CSV roundtrip failed")
        archive = destination / f"wirecraft-store-{variant}.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in sorted(folder.rglob("*")):
                if path.is_file():
                    bundle.write(path, path.relative_to(folder.parent))
        checksums.append(hashlib.sha256(archive.read_bytes()).hexdigest() + "  " + archive.name)
        print(f"{variant}: {len(locales)} locales, {SCREENSHOTS} screenshots, 9 artwork assets; {archive}")
        if variant == "all-languages":
            require(rows[0] == template[0], "Full export header changed")
            all_rows = encoded
    if update_export:
        EXPORT.write_bytes(all_rows)
        print(f"Filled original export: {EXPORT}")
    report = dict(
        locales=list(translations), screenshots=SCREENSHOTS, artworkAssets=9,
        rows=len(template) - 1, minimumApplicationVersion=manifest["minimumApplicationVersion"],
        validation="CSV structure, localized limits, image dimensions, source screenshot hashes and import paths passed",
        publication="Prepared locally; not uploaded to Partner Center or certified",
        omitted="Unused optional slots, first-submission release notes, trailers and non-desktop assets",
    )
    (destination / "validation.json").write_text(json.dumps(report, indent=2) + "\n")
    (destination / "SHA256SUMS.txt").write_text("\n".join(checksums) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=SOURCE / "dist")
    parser.add_argument("--update-export", action="store_true",
                        help="Also fill the original Partner Center export in packaging/")
    parser.add_argument("--check", action="store_true", help="Validate without rebuilding")
    args = parser.parse_args()
    if args.check:
        template, translations, manifest = load_source()
        rows = read_csv(EXPORT)
        require(rows[0] == template[0], "Original export locale header changed")
        validate_rows(rows, template, SOURCE / "assets")
        identity = store_identity(SOURCE)
        require(rows == make_rows(template, translations, manifest, template[0][4:], identity),
                "Original export is stale; rebuild with --update-export")
        print(f"Validated {len(translations)} locales and all {SCREENSHOTS + len(ART)} PNG assets")
    else:
        build(args.output.resolve(), update_export=args.update_export)


if __name__ == "__main__":
    main()
