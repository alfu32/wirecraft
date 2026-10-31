#!/usr/bin/env python3
"""Generate and check MSIX manifests carrying the reserved Microsoft Store identity.

There are no signing secrets: each release run signs with a fresh self-signed
certificate whose subject is the reserved publisher, and ships its public CER for
Trusted People installation. The Store re-signs the packages it accepts.
"""

import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

from common import ROOT, safe, sha, validate_binary

IDENTITY = ROOT / "packaging/windows/msix-identity.json"
NS = {
    "": "http://schemas.microsoft.com/appx/manifest/foundation/windows10",
    "uap": "http://schemas.microsoft.com/appx/manifest/uap/windows10",
    "uap3": "http://schemas.microsoft.com/appx/manifest/uap/windows10/3",
    "desktop": "http://schemas.microsoft.com/appx/manifest/desktop/windows10",
    "rescap": "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities",
}
for prefix, namespace in NS.items():
    ET.register_namespace(prefix, namespace)


def publisher_id(publisher):
    """Windows' 13-character publisher ID: the first 64 bits of SHA-256 over the
    UTF-16LE publisher, padded to 65 bits, in Crockford-style base32."""
    digest = hashlib.sha256(publisher.encode("utf-16-le")).digest()[:8]
    bits = int.from_bytes(digest, "big") << 1
    alphabet = "0123456789abcdefghjkmnpqrstvwxyz"
    return "".join(alphabet[(bits >> (60 - 5 * i)) & 31] for i in range(13))


def identity(path=IDENTITY):
    value = json.loads(path.read_text())
    for field in ("name", "publisher", "publisherDisplayName", "displayName"):
        if not isinstance(value.get(field), str) or not value[field].strip():
            raise ValueError("MSIX identity field missing: " + field)
    if not re.fullmatch(r"[A-Za-z0-9.-]{3,50}", value["name"]):
        raise ValueError("Invalid MSIX package name")
    if not re.fullmatch(r"CN=[^,=]+", value["publisher"]):
        raise ValueError("MSIX publisher must be a single common name")
    if not isinstance(value.get("versionMajorOffset"), int) or value["versionMajorOffset"] < 0:
        raise ValueError("Invalid MSIX version major offset")
    # The family name is copied from Partner Center's Product identity; it must be the
    # one Windows derives from the name and publisher.
    if value.get("packageFamilyName") != value["name"] + "_" + publisher_id(value["publisher"]):
        raise ValueError("MSIX package family name does not match the name and publisher")
    return value


def package_version(version, config=None):
    config = config or identity()
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("MSIX needs a numeric SemVer app version")
    fields = list(map(int, version.split(".")))
    fields[0] += config["versionMajorOffset"]
    if not 1 <= fields[0] <= 65535 or any(value > 65535 for value in fields[1:]):
        raise ValueError("MSIX version exceeds Windows package bounds")
    return ".".join(map(str, fields + [0]))


def manifest(version, arch, config=None):
    if arch not in ("x64", "arm64"):
        raise ValueError("Unsupported MSIX architecture")
    config = config or identity()

    def add(parent, tag, text=None, **attributes):
        prefix, local = tag.split(":", 1) if ":" in tag else ("", tag)
        element = ET.SubElement(parent, "{" + NS[prefix] + "}" + local, attributes)
        element.text = text
        return element

    description = "Wirecraft dataflow topology designer"
    root = ET.Element("{" + NS[""] + "}Package", IgnorableNamespaces="uap uap3 desktop rescap")
    add(root, "Identity", Name=config["name"], Publisher=config["publisher"],
        Version=package_version(version, config), ProcessorArchitecture=arch)
    properties = add(root, "Properties")
    add(properties, "DisplayName", config["displayName"])
    add(properties, "PublisherDisplayName", config["publisherDisplayName"])
    add(properties, "Description", description)
    add(properties, "Logo", "Assets\\StoreLogo.png")
    add(add(root, "Resources"), "Resource", Language="en-us")
    add(add(root, "Dependencies"), "TargetDeviceFamily", Name="Windows.Desktop",
        MinVersion="10.0.17763.0", MaxVersionTested="10.0.22621.0")
    application = add(add(root, "Applications"), "Application", Id="Wirecraft",
                      Executable="wirecraft-desktop.exe", EntryPoint="Windows.FullTrustApplication")
    add(application, "uap:VisualElements", DisplayName=config["displayName"],
        Description=description, BackgroundColor="transparent",
        Square150x150Logo="Assets\\Square150x150Logo.png", Square44x44Logo="Assets\\Square44x44Logo.png")
    extensions = add(application, "Extensions")
    file_type = add(add(extensions, "uap:Extension", Category="windows.fileTypeAssociation"),
                    "uap:FileTypeAssociation", Name="orch")
    add(add(file_type, "uap:SupportedFileTypes"), "uap:FileType", ".orch")
    alias = add(extensions, "uap3:Extension", Category="windows.appExecutionAlias",
                Executable="wirecraft-desktop.exe", EntryPoint="Windows.FullTrustApplication")
    add(add(alias, "uap3:AppExecutionAlias"), "desktop:ExecutionAlias", Alias="wirecraft-desktop.exe")
    # Windows refuses to start a packaged executable the manifest does not declare. The
    # command line is declared by its own alias on the one application: the Store refuses
    # a second, unlisted (AppListEntry="none") application without a headless-app waiver.
    cli_alias = add(extensions, "uap3:Extension", Category="windows.appExecutionAlias",
                    Executable="wirecraft.exe", EntryPoint="Windows.FullTrustApplication")
    add(add(cli_alias, "uap3:AppExecutionAlias"), "desktop:ExecutionAlias", Alias="wirecraft.exe")
    add(add(root, "Capabilities"), "rescap:Capability", Name="runFullTrust")
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def check_manifest(data, version, arch):
    expected = ET.fromstring(manifest(version, arch))
    actual = ET.fromstring(data)

    def structure(element):
        return (element.tag, element.attrib, (element.text or "").strip(),
                [structure(child) for child in element])

    if structure(actual) != structure(expected):
        raise ValueError("MSIX manifest differs from the configured identity/version")


def check_block_map(archive):
    """Independently check every payload block against the signed MSIX block map."""
    block_map = ET.fromstring(archive.read("AppxBlockMap.xml"))
    namespace = "{http://schemas.microsoft.com/appx/2010/blockmap}"
    if block_map.tag != namespace + "BlockMap" or block_map.get("HashMethod") != "http://www.w3.org/2001/04/xmlenc#sha256":
        raise ValueError("Unsupported MSIX block map")
    seen = set()
    for entry in block_map:
        name = entry.get("Name", "").replace("\\", "/")
        safe(name)
        if name in seen:
            raise ValueError("Duplicate MSIX block-map entry")
        seen.add(name)
        data = archive.read(name)
        if len(data) != int(entry.get("Size", "-1")):
            raise ValueError("MSIX block-map size mismatch: " + name)
        blocks = entry.findall(namespace + "Block")
        file_hashes = entry.findall("{http://schemas.microsoft.com/appx/2021/blockmap}FileHash")
        if len(file_hashes) > 1 or len(entry) != len(blocks) + len(file_hashes):
            raise ValueError("Unsupported MSIX block-map file entry: " + name)
        for file_hash in file_hashes:
            digest = base64.b64encode(hashlib.sha256(data).digest()).decode()
            if file_hash.get("Hash") != digest:
                raise ValueError("MSIX block-map whole-file digest mismatch: " + name)
        if len(blocks) != (len(data) + 65535) // 65536:
            raise ValueError("MSIX block-map block count mismatch: " + name)
        for offset, block in enumerate(blocks):
            digest = base64.b64encode(hashlib.sha256(data[offset * 65536:(offset + 1) * 65536]).digest()).decode()
            if block.tag != namespace + "Block" or block.get("Hash") != digest:
                raise ValueError("MSIX block-map digest mismatch: " + name)
    excluded = {"AppxBlockMap.xml", "AppxSignature.p7x", "[Content_Types].xml"}
    if "AppxMetadata/CodeIntegrity.cat" not in seen:
        excluded.add("AppxMetadata/CodeIntegrity.cat")
    if seen != {item.filename for item in archive.infolist() if not item.is_dir()} - excluded:
        raise ValueError("MSIX block-map inventory mismatch")


def check_certificate(data):
    result = subprocess.run(["openssl", "x509", "-inform", "DER", "-subject", "-nameopt", "RFC2253", "-noout"],
                            input=data, capture_output=True)
    if result.returncode or result.stdout.decode().strip() != "subject=" + identity()["publisher"]:
        raise ValueError("MSIX public certificate does not match the publisher")


def check_package(archive, version, arch, portable_manifest=None):
    if len(archive.namelist()) != len(set(archive.namelist())):
        raise ValueError("Duplicate MSIX ZIP entry")
    check_manifest(archive.read("AppxManifest.xml"), version, arch)
    signature = archive.read("AppxSignature.p7x")
    if not signature.startswith(b"PKCX") or len(signature) < 64:
        raise ValueError("MSIX signature missing or invalid")
    check_block_map(archive)
    if portable_manifest is not None:
        if json.loads(archive.read("package-manifest.json")) != portable_manifest:
            raise ValueError("MSIX payload differs from its portable source")
        for name, entry in portable_manifest["files"].items():
            safe(name)
            value = archive.read(name)
            if len(value) != entry["size"] or sha(value) != entry["sha256"]:
                raise ValueError("MSIX payload digest mismatch: " + name)
        for command in ("wirecraft-desktop.exe", "wirecraft.exe"):
            validate_binary(portable_manifest["target"], archive.read(command))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", choices=("x64", "arm64"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--identity", action="store_true",
                        help="print the identity with its computed package family name")
    args = parser.parse_args()
    if args.identity:
        print(json.dumps(identity()))
    args.output.write_bytes(manifest(args.version, args.arch))
