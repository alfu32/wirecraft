"""Manifest, package-integrity and identity contract checks.

ZIP fixtures have synthetic CMS headers. Real signing and deployment verification
runs on the native Windows CI runners, never against these test packages.
"""

import base64
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET
import zipfile

from common import sha
import msix


def certificate_fixture(directory):
    certificate = directory / "test.cer"
    subprocess.run([
        "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-sha256",
        "-subj", "/" + msix.identity()["publisher"], "-outform", "DER",
        "-out", str(certificate), "-keyout", str(directory / "test.key"),
    ], check=True, capture_output=True)
    return certificate.read_bytes()


def package_fixture(version, arch, files):
    files = dict(files, **{"AppxManifest.xml": msix.manifest(version, arch)})
    namespace = "{http://schemas.microsoft.com/appx/2010/blockmap}"
    block_map = ET.Element(namespace + "BlockMap", HashMethod="http://www.w3.org/2001/04/xmlenc#sha256")
    for name, data in files.items():
        entry = ET.SubElement(block_map, namespace + "File", Name=name.replace("/", "\\"), Size=str(len(data)))
        for offset in range(0, len(data), 65536):
            digest = base64.b64encode(bytes.fromhex(sha(data[offset:offset + 65536]))).decode()
            ET.SubElement(entry, namespace + "Block", Hash=digest)
        if len(data) > 65536:
            ET.SubElement(entry, "{http://schemas.microsoft.com/appx/2021/blockmap}FileHash",
                          Hash=base64.b64encode(bytes.fromhex(sha(data))).decode())
    files["AppxBlockMap.xml"] = ET.tostring(block_map)
    files["AppxSignature.p7x"] = b"PKCX" + b"synthetic-CMS-fixture" * 8
    files["[Content_Types].xml"] = b"<Types/>"
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for name, data in files.items():
            archive.writestr(name, data)
    return buffer.getvalue()


class MsixTests(unittest.TestCase):
    def test_publisher_ids_match_windows(self):
        # Known package family suffixes published by Windows.
        self.assertEqual(msix.publisher_id("CN=7F0CBC95-8035-4AF2-9B17-C38D58351CE5"), "tfmtd4yrk952g")
        self.assertEqual(
            msix.publisher_id("CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"),
            "8wekyb3d8bbwe",
        )
        identity = msix.identity()
        self.assertEqual(identity["packageFamilyName"], identity["name"] + "_" + msix.publisher_id(identity["publisher"]))

    def test_identity_both_architectures_and_version_mapping(self):
        identity = msix.identity()
        for arch in ("x64", "arm64"):
            data = msix.manifest("0.41.1", arch)
            root = ET.fromstring(data)
            name = "{" + msix.NS[""] + "}"
            package = root.find(name + "Identity")
            self.assertEqual(package.get("Name"), identity["name"])
            self.assertEqual(package.get("Publisher"), identity["publisher"])
            self.assertEqual(package.get("ProcessorArchitecture"), arch)
            self.assertEqual(package.get("Version"), "1.41.1.0")
            applications = root.findall(name + "Applications/" + name + "Application")
            self.assertEqual([a.get("Executable") for a in applications], ["wirecraft-desktop.exe", "wirecraft.exe"])
            # Undeclared packaged executables cannot be started (Access is denied).
            aliases = root.findall(".//{" + msix.NS["desktop"] + "}ExecutionAlias")
            self.assertEqual([a.get("Alias") for a in aliases], ["wirecraft-desktop.exe", "wirecraft.exe"])
            self.assertEqual(applications[1].get("{" + msix.NS["desktop4"] + "}Subsystem"), "console")
            self.assertEqual(applications[1].get("{" + msix.NS["desktop4"] + "}SupportsMultipleInstances"), "true")
            self.assertIn(b".orch", data)
            self.assertIn(b"windows.appExecutionAlias", data)
            msix.check_manifest(data, "0.41.1", arch)
        self.assertEqual(msix.package_version("1.0.0"), "2.0.0.0")
        for version in ("v1.0.0", "0.01.0", "65535.0.0", "0.65536.0", "0.0.65536"):
            with self.assertRaises(ValueError):
                msix.package_version(version)

    def test_rejects_an_identity_without_a_single_common_name(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "identity.json"
            value = json.loads(msix.IDENTITY.read_text())
            value["publisher"] = "CN=Wirecraft, O=Other"
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "single common name"):
                msix.identity(path)

    def test_rejects_wrong_publisher_architecture_and_modified_payload_blocks(self):
        data = msix.manifest("0.41.1", "x64")
        wrong = data.replace(msix.identity()["publisher"].encode(), b"CN=Other")
        with self.assertRaisesRegex(ValueError, "configured identity"):
            msix.check_manifest(wrong, "0.41.1", "x64")
        with self.assertRaises(ValueError):
            msix.check_manifest(data, "0.41.1", "arm64")
        package = package_fixture("0.41.1", "x64", {"wirecraft-desktop.exe": b"payload" * 20000})
        with zipfile.ZipFile(io.BytesIO(package)) as archive:
            msix.check_package(archive, "0.41.1", "x64")
            changed = {name: archive.read(name) for name in archive.namelist()}
        block_map = ET.fromstring(changed["AppxBlockMap.xml"])
        file_hash = block_map.find(".//{http://schemas.microsoft.com/appx/2021/blockmap}FileHash")
        self.assertIsNotNone(file_hash)
        file_hash.set("Hash", "incorrect")
        altered = io.BytesIO()
        with zipfile.ZipFile(altered, "w") as archive:
            for name, value in changed.items():
                archive.writestr(name, ET.tostring(block_map) if name == "AppxBlockMap.xml" else value)
        with zipfile.ZipFile(io.BytesIO(altered.getvalue())) as archive:
            with self.assertRaisesRegex(ValueError, "whole-file digest"):
                msix.check_package(archive, "0.41.1", "x64")
        changed["wirecraft-desktop.exe"] = b"modified" * 17500
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            for name, value in changed.items():
                archive.writestr(name, value)
        with zipfile.ZipFile(io.BytesIO(output.getvalue())) as archive:
            with self.assertRaisesRegex(ValueError, "block-map"):
                msix.check_package(archive, "0.41.1", "x64")


if __name__ == "__main__":
    unittest.main()
