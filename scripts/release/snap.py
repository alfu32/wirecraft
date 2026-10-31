#!/usr/bin/env python3
"""Pack a desktop Snap including its audited native dependencies."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from common import ROOT, payload, publish, sha

# `wirecraft` is the command line; `wirecraft.desktop` starts the application.
APPS = {"wirecraft": "wirecraft", "desktop": "wirecraft-desktop"}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--archive", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="wirecraft-snap-") as temp:
        stage = Path(temp)
        package, manifest = payload(a.archive, stage / "input")
        if "linux" not in manifest["target"]:
            raise ValueError("Snap needs a Linux payload")
        arch = "arm64" if manifest["target"].startswith("aarch64") else "amd64"
        root = stage / "snap"
        (root / "opt").mkdir(parents=True)
        shutil.copytree(package, root / "opt/wirecraft")
        (root / "bin").mkdir()
        (root / "gpu-2404").mkdir()
        for app, command in APPS.items():
            (root / "bin" / app).write_text(
                '#!/bin/sh\nexport XDG_CONFIG_HOME="$SNAP_USER_COMMON/.config"\n'
                'export XDG_CACHE_HOME="$SNAP_USER_COMMON/.cache"\n'
                'export XDG_STATE_HOME="$SNAP_USER_COMMON/.local/state"\n'
                'export WIRECRAFT_EXEC_WRAPPER="$SNAP/gpu-2404/bin/gpu-2404-provider-wrapper"\n'
                'exec "$SNAP/opt/wirecraft/' + command + '" "$@"\n'
            )
            (root / "bin" / app).chmod(0o755)
        (root / "meta/gui").mkdir(parents=True)
        shutil.copyfile(
            ROOT / "packaging/wirecraft.png", root / "meta/gui/wirecraft.png"
        )
        desktop = (
            (ROOT / "packaging/wirecraft.desktop")
            .read_text()
            .replace("Exec=wirecraft-desktop ", "Exec=wirecraft.desktop ")
            .replace("Icon=wirecraft", "Icon=${SNAP}/meta/gui/wirecraft.png")
        )
        (root / "meta/gui/wirecraft.desktop").write_text(desktop)
        plugs = "[home, removable-media, desktop, desktop-legacy, wayland, x11, opengl, network, network-bind, gpu-2404]"
        (root / "meta/snap.yaml").write_text(
            """name: wirecraft
version: '{version}'
summary: Dataflow topology designer that generates programs
description: |
  Wirecraft GPUI desktop and command line for Threadworks-compatible .orch
  projects. Strict confinement: host compilers and language servers (clang,
  clangd, rust-analyzer, Go, Node.js) are not reachable from the Snap.
base: core24
architectures: [{arch}]
plugs:
  gpu-2404:
    interface: content
    content: gpu-2404
    default-provider: mesa-2404
    target: $SNAP/gpu-2404
confinement: strict
grade: stable
license: MIT OR Apache-2.0
apps:
  wirecraft:
    command: bin/wirecraft
    plugs: {plugs}
  desktop:
    command: bin/desktop
    plugs: {plugs}
""".format(version=manifest["version"], arch=arch, plugs=plugs)
        )
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
        for path in sorted(root.rglob("*"), reverse=True):
            os.utime(path, (epoch, epoch))
        subprocess.run(
            ["snap", "pack", str(root), str(a.output.resolve()), "--compression=xz"],
            check=True,
        )
        artifact = a.output / (
            "wirecraft_" + manifest["version"] + "_" + arch + ".snap"
        )
        if not artifact.is_file():
            raise ValueError("Snap artifact missing")
        publish(
            artifact,
            dict(
                format="snap",
                target=manifest["target"],
                base="core24",
                confinement="strict",
                grade="stable",
                minimumGlibc=manifest["minimumGlibc"],
                payloadArchiveSha256=sha(a.archive.read_bytes()),
            ),
        )


if __name__ == "__main__":
    main()
