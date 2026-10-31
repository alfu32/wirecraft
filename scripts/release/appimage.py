#!/usr/bin/env python3
"""Assemble AppImages with the same checksum-pinned tools/runtime as Meshcraft."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import urllib.request
from common import ROOT, payload, publish, sha


def download(entry, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.exists():
        with urllib.request.urlopen(entry["url"], timeout=60) as response:
            data = response.read(40 * 1024 * 1024 + 1)
        if len(data) > 40 * 1024 * 1024 or sha(data) != entry["sha256"]:
            raise ValueError("Installer input checksum/size mismatch")
        destination.write_bytes(data)
    if sha(destination.read_bytes()) != entry["sha256"]:
        raise ValueError("Cached installer input checksum mismatch")
    destination.chmod(0o755)
    return destination.resolve()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--archive", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=True)
    inputs = json.loads((ROOT / "packaging/appimage-inputs.json").read_text())
    with tempfile.TemporaryDirectory(prefix="wirecraft-appimage-") as temp:
        stage = Path(temp)
        package, manifest = payload(a.archive, stage / "input")
        if "linux" not in manifest["target"]:
            raise ValueError("AppImage needs a Linux payload")
        arch = manifest["target"].split("-")[0]
        selected = inputs["aarch64"] if arch == "aarch64" else inputs
        cache = ROOT / "target/installer-tools"
        tool = download(
            selected["tool"], cache / ("appimagetool-" + arch + ".AppImage")
        )
        runtime = download(selected["runtime"], cache / ("runtime-" + arch))
        app = stage / "Wirecraft.AppDir"
        (app / "usr/lib").mkdir(parents=True)
        shutil.copytree(package, app / "usr/lib/wirecraft")
        shutil.copyfile(ROOT / "packaging/linux/AppRun", app / "AppRun")
        (app / "AppRun").chmod(0o755)
        shutil.copyfile(ROOT / "packaging/wirecraft.desktop", app / "wirecraft.desktop")
        shutil.copyfile(ROOT / "packaging/wirecraft.png", app / "wirecraft.png")
        shutil.copyfile(ROOT / "packaging/wirecraft.png", app / ".DirIcon")
        for entry in inputs["notices"]:
            source = ROOT / "packaging" / entry["file"]
            if sha(source.read_bytes()) != entry["sha256"]:
                raise ValueError("Runtime notice changed")
            target = (
                app
                / "usr/lib/wirecraft/notices/appimage"
                / source.relative_to(ROOT / "packaging/licenses/appimage-runtime")
            )
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        (app / "usr/lib/wirecraft/notices/appimage/manifest.json").write_text(
            json.dumps(inputs, sort_keys=True, indent=2) + "\n"
        )
        subprocess.run(
            ["desktop-file-validate", str(app / "wirecraft.desktop")], check=True
        )
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
        for path in sorted(app.rglob("*"), reverse=True):
            os.utime(path, (epoch, epoch))
        os.utime(app, (epoch, epoch))
        artifact = a.output / (
            "wirecraft-" + manifest["version"] + "-" + arch + ".AppImage"
        )
        env = dict(
            os.environ,
            ARCH=arch,
            SOURCE_DATE_EPOCH=str(epoch),
            APPIMAGE_EXTRACT_AND_RUN="1",
        )
        subprocess.run(
            [
                str(tool),
                "--runtime-file",
                str(runtime),
                "--no-appstream",
                "--comp",
                "zstd",
                "--mksquashfs-opt=-processors",
                "--mksquashfs-opt=2",
                str(app),
                str(artifact.resolve()),
            ],
            env=env,
            check=True,
        )
        artifact.chmod(0o755)
        publish(
            artifact,
            dict(
                format="appimage",
                target=manifest["target"],
                minimumGlibc=manifest["minimumGlibc"],
                payloadArchiveSha256=sha(a.archive.read_bytes()),
                toolSha256=selected["tool"]["sha256"],
                runtimeSha256=selected["runtime"]["sha256"],
            ),
        )


if __name__ == "__main__":
    main()
