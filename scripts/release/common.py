"""Shared metadata and payload checks for release packaging (Python 3.8+).

Ported from Meshcraft's release scripts; Wirecraft ships two executables per target:
the `wirecraft-desktop` application (which also runs its own backend) and the
`wirecraft` command line.
"""

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import struct
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = {
    "x86_64-unknown-linux-gnu": ("x86_64.AppImage", "amd64.snap", "amd64.deb", "1.x86_64.rpm"),
    "aarch64-unknown-linux-gnu": ("aarch64.AppImage", "arm64.snap", "arm64.deb", "1.aarch64.rpm"),
    "x86_64-pc-windows-msvc": ("windows-x64.msi", "windows-x64.msix", "windows-x64.cer"),
    "aarch64-pc-windows-msvc": ("windows-arm64.msi", "windows-arm64.msix", "windows-arm64.cer"),
    "x86_64-apple-darwin": ("macos-x86_64.dmg",),
    "aarch64-apple-darwin": ("macos-aarch64.dmg",),
}
# The application first: launchers, shortcuts and bundles start it.
COMMANDS = ("wirecraft-desktop", "wirecraft")


def version(root=ROOT):
    # Require the literal workspace version that every crate inherits.
    text = (root / "Cargo.toml").read_text()
    package = re.search(r"(?ms)^\[workspace\.package\]\s*\n(.*?)(?=^\[|\Z)", text)
    match = re.search(
        r'(?m)^version\s*=\s*"([^"]+)"\s*$', package[1] if package else ""
    )
    if not match or not re.fullmatch(
        r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", match[1]
    ):
        raise ValueError("Workspace needs a literal X.Y.Z release version")
    return match[1]


def tag(app_version):
    """Wirecraft tags releases `vX.Y.Z`."""
    return "v" + app_version


def executable(target, command):
    """Path of a command inside a portable payload."""
    if "linux" in target:
        return "bin/" + command
    return command + ".exe" if "windows" in target else command


def revision():
    return (
        os.environ.get("WIRECRAFT_BUILD_SHA")
        or subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip()
    )


def sha(data):
    return hashlib.sha256(data).hexdigest()


def publish(path, metadata=None):
    digest = sha(path.read_bytes())
    Path(str(path) + ".sha256").write_text(digest + "  " + path.name + "\n")
    if metadata is not None:
        metadata.update(
            schemaVersion=1, artifact=path.name, sha256=digest, size=path.stat().st_size
        )
        Path(str(path) + ".manifest.json").write_text(
            json.dumps(metadata, sort_keys=True, indent=2) + "\n"
        )
    print(path)


def safe(name):
    path = PurePosixPath(name)
    if (
        path.is_absolute()
        or not path.parts
        or ".." in path.parts
        or "\\" in name
        or ":" in name
    ):
        raise ValueError("Unsafe package path: " + name)
    return path


def payload(archive, directory):
    directory = Path(directory)
    with tarfile.open(archive) as source:
        members = source.getmembers()
        names = [m.name for m in members]
        if (
            not members
            or len(set(names)) != len(names)
            or not all(m.isfile() for m in members)
        ):
            raise ValueError("Payload must contain unique regular files")
        roots = {safe(name).parts[0] for name in names}
        if len(roots) != 1:
            raise ValueError("Payload must have one root")
        # Write regular files explicitly; do not permit links or archive extraction traversal.
        for member in members:
            target = directory.joinpath(*safe(member.name).parts)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.extractfile(member).read())
            target.chmod(0o755 if member.mode & 0o111 else 0o644)
    package = directory / roots.pop()
    manifest = json.loads((package / "package-manifest.json").read_text())
    actual = {
        p.relative_to(package).as_posix() for p in package.rglob("*") if p.is_file()
    }
    if actual != set(manifest["files"]) | {"package-manifest.json"}:
        raise ValueError("Payload differs from its manifest inventory")
    for name, entry in manifest["files"].items():
        p = package.joinpath(*safe(name).parts)
        if p.stat().st_size != entry["size"] or sha(p.read_bytes()) != entry["sha256"]:
            raise ValueError("Payload digest differs: " + name)
        if bool(p.stat().st_mode & 0o111) != entry["executable"]:
            raise ValueError("Payload mode differs: " + name)
    return package, manifest


def validate_binary(target, data):
    """Check native executable kind/CPU independently of the artifact filename."""
    arch = target.split("-")[0]
    if "linux" in target:
        valid = (
            len(data) >= 20
            and data[:6] == b"\x7fELF\x02\x01"
            and struct.unpack_from("<H", data, 18)[0]
            == (183 if arch == "aarch64" else 62)
        )
    elif "windows" in target:
        offset = struct.unpack_from("<I", data, 60)[0] if len(data) >= 64 else len(data)
        valid = (
            data[:2] == b"MZ"
            and offset + 6 <= len(data)
            and data[offset : offset + 4] == b"PE\0\0"
            and struct.unpack_from("<H", data, offset + 4)[0]
            == (0xAA64 if arch == "aarch64" else 0x8664)
        )
    else:
        valid = (
            len(data) >= 8
            and data[:4] == b"\xcf\xfa\xed\xfe"
            and struct.unpack_from("<I", data, 4)[0]
            == (0x100000C if arch == "aarch64" else 0x1000007)
        )
    if not valid:
        raise ValueError(
            "Executable format/architecture differs from target: " + target
        )
