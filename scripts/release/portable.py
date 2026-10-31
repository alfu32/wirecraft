#!/usr/bin/env python3
"""Package manifested native binaries, notices and curated examples, without user data."""

import argparse
import gzip
import io
import json
import os
from pathlib import Path
import tarfile
import zipfile
from common import (
    COMMANDS,
    ROOT,
    TARGETS,
    executable,
    publish,
    revision,
    sha,
    tag,
    validate_binary,
    version,
)

# Named explicitly: examples/ also holds the user's own projects and generated output.
EXAMPLES = [
    "webserver.orch",
    "webserver-component.orch",
    "webserver-go.orch",
    "webserver-kotlin.orch",
    "webserver-nodejs.orch",
    "webserver-php.orch",
    "webserver-rust.orch",
    "webserver-systemjs.orch",
]
DOCUMENTS = {
    "README.md": "README.md",
    "LICENSE-MIT": "LICENSE-MIT",
    "LICENSE-APACHE": "LICENSE-APACHE",
    "packaging/README.md": "packaging/README.md",
    "wirecraft.png": "packaging/wirecraft.png",
    "notices/Monaspace-OFL.txt": "assets/fonts/OFL.txt",
    "notices/gpui-component-Apache-2.0.txt": "vendor/gpui-component/LICENSE-APACHE",
    "notices/gpui-component-patches.md": "vendor/gpui-component/WIRECRAFT-PATCH.md",
}


def assemble(target, binaries, output, native=None):
    """`binaries` is the directory holding the built `wirecraft-desktop` and `wirecraft`."""
    if target not in TARGETS:
        raise ValueError("Unsupported target: " + target)
    files = {}

    def add(name, path, executable_file=False):
        if path.is_symlink() or not path.is_file():
            raise ValueError("Missing or symlinked package input: " + str(path))
        files[name] = (path.read_bytes(), executable_file)

    linux = "linux" in target
    windows = "windows" in target
    for command in COMMANDS:
        add(
            executable(target, command),
            binaries / (command + (".exe" if windows else "")),
            not windows,
        )
    if linux:
        if native is None:
            raise ValueError("Linux packages require an audited native bundle")
        report = json.loads((native / "native-manifest.json").read_text())
        if report["target"] != target:
            raise ValueError("Native bundle target mismatch")
        # The native bundle's executables are the rpath-patched ones and replace the originals.
        for path in sorted(native.rglob("*")):
            if path.is_file():
                add(
                    path.relative_to(native).as_posix(),
                    path,
                    bool(path.stat().st_mode & 0o111),
                )
        # One launcher per command; it starts bin/<its own name> with the bundled libraries.
        for command in COMMANDS:
            add(command, ROOT / "packaging/linux/launcher", True)
        add("xkb-runtime.sh", ROOT / "packaging/linux/xkb-runtime.sh")
    for name, source in DOCUMENTS.items():
        add(name, ROOT / source)
    for name in EXAMPLES:
        add("examples/" + name, ROOT / "examples" / name)
    for command in COMMANDS:
        validate_binary(target, files[executable(target, command)][0])
    app_version = version()
    manifest = dict(
        schemaVersion=1,
        target=target,
        version=app_version,
        tag=tag(app_version),
        sourceRevision=revision(),
        assets="embedded in executable",
        files={
            name: dict(sha256=sha(data), size=len(data), executable=x)
            for name, (data, x) in sorted(files.items())
        },
    )
    if linux:
        manifest["minimumGlibc"] = report["minimumGlibc"]
    files["package-manifest.json"] = (
        (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode(),
        False,
    )
    name = "wirecraft-" + app_version + "-" + target
    output.mkdir(parents=True, exist_ok=True)
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
    archive = output / (name + ".tar.gz")
    with archive.open("wb") as raw, gzip.GzipFile(
        filename="", fileobj=raw, mode="wb", mtime=epoch
    ) as compressed:
        with tarfile.open(
            fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT
        ) as tar:
            for relative, (data, x) in sorted(files.items()):
                info = tarfile.TarInfo(name + "/" + relative)
                info.size = len(data)
                info.mtime = epoch
                info.mode = 0o755 if x else 0o644
                tar.addfile(info, io.BytesIO(data))
    zip_path = output / (name + ".zip")
    with zipfile.ZipFile(
        zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as zipped:
        for relative, (data, x) in sorted(files.items()):
            info = zipfile.ZipInfo(
                name + "/" + relative, date_time=(2026, 1, 1, 0, 0, 0)
            )
            info.external_attr = (0o100755 if x else 0o100644) << 16
            zipped.writestr(
                info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9
            )
    for path in [archive, zip_path]:
        publish(path)
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument(
        "--binaries",
        type=Path,
        required=True,
        help="directory with the built wirecraft-desktop and wirecraft executables",
    )
    parser.add_argument("--output", type=Path, default=ROOT / "target/release-assets")
    parser.add_argument("--native", type=Path)
    a = parser.parse_args()
    assemble(a.target, a.binaries, a.output, a.native)
