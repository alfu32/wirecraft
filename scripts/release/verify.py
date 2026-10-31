#!/usr/bin/env python3
"""Require every native target, installer, digest and matching portable payload before publication."""

import argparse
import io
import json
from pathlib import Path
import tarfile
import zipfile
from common import COMMANDS, ROOT, TARGETS, executable, safe, sha, tag, validate_binary
from linux_packages import ARCHITECTURES, check_package
import msix


def expected_assets(version):
    names = set()
    for target, installers in TARGETS.items():
        base = "wirecraft-" + version + "-" + target
        names.update([base + ".zip", base + ".tar.gz"])
        for suffix in installers:
            names.add(
                "wirecraft_" + version + "-1_" + suffix
                if suffix.endswith(".deb")
                else "wirecraft_" + version + "_" + suffix
                if suffix.endswith(".snap")
                else "wirecraft-" + version + "-" + suffix
            )
    return names


def check_portable(data, name, version, revision):
    root = name[:-7] if name.endswith(".tar.gz") else name[:-4]
    target = root[len("wirecraft-" + version + "-") :]
    contents = {}
    if name.endswith(".tar.gz"):
        with tarfile.open(fileobj=io.BytesIO(data)) as a:
            for m in a.getmembers():
                safe(m.name)
                if not m.isfile() or m.name in contents:
                    raise ValueError("Invalid archive member")
                contents[m.name] = a.extractfile(m).read()
    else:
        with zipfile.ZipFile(io.BytesIO(data)) as a:
            for m in a.infolist():
                safe(m.filename)
                if m.is_dir() or m.filename in contents:
                    raise ValueError("Invalid ZIP member")
                contents[m.filename] = a.read(m)
    manifest = json.loads(contents[root + "/package-manifest.json"])
    if (
        manifest["version"],
        manifest["tag"],
        manifest["target"],
        manifest["sourceRevision"],
    ) != (version, tag(version), target, revision):
        raise ValueError("Payload release source mismatch")
    if set(contents) != {root + "/" + p for p in manifest["files"]} | {
        root + "/package-manifest.json"
    }:
        raise ValueError("Portable inventory mismatch")
    for command in COMMANDS:
        path = executable(target, command)
        if path not in manifest["files"]:
            raise ValueError("Application binary missing: " + command)
        validate_binary(target, contents[root + "/" + path])
    for p, entry in manifest["files"].items():
        safe(p)
        value = contents[root + "/" + p]
        if sha(value) != entry["sha256"] or len(value) != entry["size"]:
            raise ValueError("Portable file digest mismatch: " + p)
    if "linux" in target and tuple(map(int, manifest["minimumGlibc"].split("."))) > (
        2,
        31,
    ):
        raise ValueError("Linux payload exceeds glibc ceiling")


def verify(directory, version, revision):
    expected = expected_assets(version)
    actual = {
        p.name
        for p in directory.iterdir()
        if p.is_file() and not p.name.endswith((".sha256", ".manifest.json"))
    }
    if actual != expected:
        raise ValueError(
            "Release matrix differs: missing="
            + str(sorted(expected - actual))
            + " unexpected="
            + str(sorted(actual - expected))
        )
    checksums = []
    for name in sorted(expected):
        data = (directory / name).read_bytes()
        if not data:
            raise ValueError("Empty release artifact: " + name)
        digest = sha(data)
        if (directory / (name + ".sha256")).read_text().strip() != digest + "  " + name:
            raise ValueError("Invalid checksum: " + name)
        checksums.append(digest + "  " + name)
        if name.endswith((".zip", ".tar.gz")):
            check_portable(data, name, version, revision)
        elif name.endswith(".msix"):
            arch = "arm64" if name.endswith("-arm64.msix") else "x64"
            target = "aarch64-pc-windows-msvc" if arch == "arm64" else "x86_64-pc-windows-msvc"
            certificate = directory / (name[:-5] + ".cer")
            metadata = json.loads((directory / (name + ".manifest.json")).read_text(encoding="utf-8-sig"))
            identity = msix.identity()
            required = dict(
                schemaVersion=1, format="msix", version=version,
                packageVersion=msix.package_version(version), architecture=arch,
                sourceRevision=revision, artifact=name, signed=True,
                identityName=identity["name"], publisher=identity["publisher"],
                publisherDisplayName=identity["publisherDisplayName"], packageFamilyName=identity["packageFamilyName"],
                certificateSha256=sha(certificate.read_bytes()), sha256=digest, size=len(data),
            )
            if any(metadata.get(key) != value for key, value in required.items()):
                raise ValueError("MSIX signing/source metadata mismatch")
            portable_name = "wirecraft-" + version + "-" + target
            with zipfile.ZipFile(directory / (portable_name + ".zip")) as portable:
                original = json.loads(portable.read(portable_name + "/package-manifest.json"))
            with zipfile.ZipFile(io.BytesIO(data)) as a:
                msix.check_package(a, version, arch, original)
        elif name.endswith(".cer"):
            msix.check_certificate(data)
        elif name.endswith(".msi") and not data.startswith(
            bytes.fromhex("d0cf11e0a1b11ae1")
        ):
            raise ValueError("Invalid MSI")
        elif name.endswith(".AppImage") and (
            data[:4] != b"\x7fELF" or data[8:11] != b"AI\x02"
        ):
            raise ValueError("Invalid AppImage")
        elif name.endswith(".snap") and data[:4] != b"hsqs":
            raise ValueError("Invalid Snap")
        elif name.endswith((".deb", ".rpm")):
            target = next(
                target for target, arches in ARCHITECTURES.items()
                if name.endswith("_" + arches[0] + ".deb")
                or name.endswith("." + arches[1] + ".rpm")
            )
            metadata = json.loads((directory / (name + ".manifest.json")).read_text())
            archive = directory / ("wirecraft-" + version + "-" + target + ".tar.gz")
            required = dict(
                schemaVersion=1, format=name.rsplit(".", 1)[1], version=version,
                packageRevision=1, target=target, sourceRevision=revision,
                payloadArchiveSha256=sha(archive.read_bytes()), artifact=name,
                sha256=digest, size=len(data),
            )
            if any(metadata.get(key) != value for key, value in required.items()):
                raise ValueError("Native package provenance mismatch: " + name)
            installed = check_package(directory / name, version, revision, target)
            with tarfile.open(archive) as portable:
                root = archive.name[:-7]
                original = json.load(portable.extractfile(root + "/package-manifest.json"))
            if installed != original:
                raise ValueError("Native package differs from its portable payload: " + name)
        elif name.endswith(".dmg") and data[-512:-508] != b"koly":
            raise ValueError("Invalid DMG")
    return "\n".join(checksums) + "\n"


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--directory", type=Path, required=True)
    p.add_argument("--version", required=True)
    p.add_argument("--sha", required=True)
    a = p.parse_args()
    checksums = verify(a.directory, a.version, a.sha)
    (ROOT / "target").mkdir(exist_ok=True)
    (ROOT / "target/SHA256SUMS.txt").write_text(checksums)
    (ROOT / "target/release-notes.md").write_text(
        "Wirecraft " + a.version + "\n\nSource: `" + a.sha + "`.\n\n"
        "Portable ZIP/tar.gz archives for Linux, Windows and macOS on x86_64 and ARM64, each with the "
        "`wirecraft-desktop` application and the `wirecraft` command line; "
        "AppImage/Snap/DEB/RPM, MSI/MSIX and DMG installers. Linux binaries target glibc 2.27 or older. "
        "DEB and RPM packages use the audited native payload and run without Snap confinement. "
        "The strictly confined Snap cannot reach host compilers or language servers (clang, clangd, "
        "rust-analyzer, Go, Node.js), so code checks and generated-program builds need another package.\n\n"
        "DEB/RPM packages are unsigned GitHub release downloads; APT/RPM repository publishing is not configured.\n\n"
        "Windows MSIX packages carry the reserved Microsoft Store identity and are signed with a self-signed "
        "certificate created for this release; install the matching public CER into Trusted People to sideload "
        "them. MSI packages are unsigned. "
        "macOS bundles are ad-hoc signed, without Developer ID signing or notarization. "
        "Native GUI testing on Windows/macOS and ARM64 remains to be qualified.\n"
    )
    print(
        "Verified "
        + str(len(expected_assets(a.version)))
        + " primary release artifacts"
    )
