#!/usr/bin/env python3
"""Bundle non-driver ELF dependencies and enforce the declared glibc ceiling."""

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
from common import COMMANDS, ROOT, sha

# The OS supplies glibc, its loader, and the GPU/GL/Vulkan driver stack.
SYSTEM = {
    "libc.so.6",
    "libm.so.6",
    "libpthread.so.0",
    "libdl.so.2",
    "librt.so.1",
    "libresolv.so.2",
    "libutil.so.1",
    "ld-linux-x86-64.so.2",
    "ld-linux-aarch64.so.1",
    "libGL.so.1",
    "libEGL.so.1",
    "libGLX.so.0",
    "libGLdispatch.so.0",
    "libOpenGL.so.0",
    "libvulkan.so.1",
    "libgcc_s.so.1",
    "libstdc++.so.6",
}

XKB_PAIR = {"libxkbcommon.so.0", "libxkbcommon-x11.so.0"}


def library_path(name):
    return ("lib/xkb/" if name in XKB_PAIR else "lib/") + name


def glibc(path):
    text = subprocess.check_output(["readelf", "--version-info", str(path)], text=True)
    versions = re.findall(r"GLIBC_(\d+(?:\.\d+)+)", text)
    return max(versions, key=lambda v: tuple(map(int, v.split("."))), default="0.0")


def bundle(binaries, target, output, ceiling):
    if target not in ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"):
        raise ValueError("Native dependency bundling requires Linux GNU")
    if sorted(binaries) != sorted(COMMANDS):
        raise ValueError("Bundle needs exactly the executables " + ", ".join(COMMANDS))
    expected = (
        "AArch64" if target.startswith("aarch64") else "Advanced Micro Devices X86-64"
    )
    for binary in binaries.values():
        machine = subprocess.check_output(["readelf", "-h", str(binary)], text=True)
        if expected not in machine:
            raise ValueError("ELF architecture differs from release target")
    cache = subprocess.check_output(["ldconfig", "-p"], text=True)
    paths = {}
    for line in cache.splitlines():
        match = re.match(r"\s*(\S+) \(.*\) => (.*)", line)
        if match:
            paths.setdefault(match[1], Path(match[2]))
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("Native bundle output must be empty")
    pending = [("bin/" + name, path) for name, path in sorted(binaries.items())]
    # These are also loaded dynamically by GPUI/font-kit on Linux.
    for name in [
        "libfontconfig.so.1",
        "libfreetype.so.6",
        "libxkbcommon.so.0",
        "libxkbcommon-x11.so.0",
        "libX11-xcb.so.1",
        "libXcursor.so.1",
        "libXrandr.so.2",
        "libXinerama.so.1",
        "libXi.so.6",
        "libXrender.so.1",
    ]:
        if name not in paths:
            raise ValueError("Missing dynamic runtime dependency: " + name)
        pending.append((library_path(name), paths[name]))
    seen = set()
    external = set()
    records = []
    while pending:
        name, source = pending.pop()
        if name in seen:
            continue
        seen.add(name)
        dest = output / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, dest)
        dest.chmod(0o755)
        subprocess.run(
            [
                "patchelf",
                "--set-rpath",
                "$ORIGIN/../lib" if name.startswith("bin/") else "$ORIGIN",
                str(dest),
            ],
            check=True,
        )
        required = glibc(dest)
        if tuple(map(int, required.split("."))) > tuple(map(int, ceiling.split("."))):
            raise ValueError(
                name + " requires GLIBC_" + required + ", newer than " + ceiling
            )
        needed = re.findall(
            r"Shared library: \[(.*?)\]",
            subprocess.check_output(["readelf", "-d", str(dest)], text=True),
        )
        records.append(
            dict(
                file=name,
                minimumGlibc=required,
                needed=needed,
                sha256=sha(dest.read_bytes()),
            )
        )
        if name.startswith("lib/"):
            owner = None
            for candidate in [source, source.resolve()]:
                result = subprocess.run(
                    ["dpkg-query", "-S", str(candidate)], text=True, capture_output=True
                )
                if result.returncode == 0:
                    owner = result.stdout.split(": ", 1)[0].split(":")[0]
                    break
            notice = Path("/usr/share/doc") / (owner or "") / "copyright"
            if not owner or not notice.is_file():
                raise ValueError("Missing native library notice: " + str(source))
            dest_notice = output / "notices/native" / owner / "copyright"
            dest_notice.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(notice, dest_notice)
        for dependency in needed:
            if dependency in SYSTEM:
                external.add(dependency)
            else:
                if dependency not in paths:
                    raise ValueError("Unresolved library " + dependency)
                pending.append((library_path(dependency), paths[dependency]))
    # The private fallback pair also needs locale data understood by that version.
    # XLOCALEDIR still lets explicit and per-user Compose files take precedence.
    shutil.copytree("/usr/share/X11/locale", output / "share/X11/locale", symlinks=False)
    shutil.copytree("/usr/share/X11/xkb", output / "share/X11/xkb", symlinks=False)
    for package in ("libx11-data", "xkb-data"):
        (output / "notices/native" / package).mkdir(parents=True, exist_ok=True)
        shutil.copyfile(
            "/usr/share/doc/" + package + "/copyright",
            output / "notices/native" / package / "copyright",
        )
    # A minimal font set/config also works inside the isolated Snap runtime.
    fonts = Path("/usr/share/fonts/truetype/dejavu")
    if not fonts.is_dir():
        raise ValueError("fonts-dejavu-core is required for native packaging")
    shutil.copytree(fonts, output / "share/fonts")
    (output / "etc/fonts").mkdir(parents=True)
    shutil.copyfile(
        ROOT / "packaging/linux/fonts.conf", output / "etc/fonts/fonts.conf"
    )
    (output / "notices/native/fonts-dejavu-core").mkdir(parents=True)
    shutil.copyfile(
        "/usr/share/doc/fonts-dejavu-core/copyright",
        output / "notices/native/fonts-dejavu-core/copyright",
    )
    shutil.copytree(
        "/usr/share/common-licenses",
        output / "notices/native/common-licenses",
        symlinks=False,
    )
    report = dict(
        schemaVersion=1,
        target=target,
        buildGlibcCeiling=ceiling,
        minimumGlibc=max(
            (r["minimumGlibc"] for r in records),
            key=lambda v: tuple(map(int, v.split("."))),
        ),
        externalSystemLibraries=sorted(external),
        elf=sorted(records, key=lambda r: r["file"]),
    )
    (output / "native-manifest.json").write_text(
        json.dumps(report, sort_keys=True, indent=2) + "\n"
    )
    print(
        "Bundled "
        + str(len(records))
        + " ELF files; minimum glibc "
        + report["minimumGlibc"]
    )


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--binary",
        action="append",
        required=True,
        metavar="NAME=PATH",
        help="an executable to bundle, once per command",
    )
    p.add_argument("--target", required=True)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--max-glibc", default="2.27")
    a = p.parse_args()
    binaries = dict(entry.split("=", 1) for entry in a.binary)
    bundle({k: Path(v) for k, v in binaries.items()}, a.target, a.output, a.max_glibc)
