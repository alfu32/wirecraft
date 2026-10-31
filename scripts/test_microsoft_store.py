"""Guard Partner Center import metadata, localization and folder portability."""

import copy
import csv
import io
import json
from pathlib import Path
import tempfile
import unittest

import microsoft_store as store


class StoreListingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.template, cls.translations, cls.manifest = store.load_source()
        cls.identity = store.store_identity(store.SOURCE)
        cls.rows = store.make_rows(
            cls.template, cls.translations, cls.manifest,
            cls.template[0][4:], cls.identity,
        )

    def test_all_export_locales_and_field_ids_survive(self):
        self.assertEqual(len(self.translations), 32)
        self.assertEqual(self.rows[0], self.template[0])
        for row, original in zip(self.rows[1:], self.template[1:]):
            if original and any(original):
                self.assertEqual(row[:3], original[:3])
        store.validate_rows(self.rows, self.template, store.SOURCE / "assets")

    def test_csv_unicode_quotes_and_multiline_roundtrip(self):
        content = [["English", "العربية", "中文", 'Text with "quotes", and commas\nSecond line']]
        encoded = store.encode_csv(content)
        self.assertTrue(encoded.startswith(b"\xef\xbb\xbf"))
        self.assertEqual(list(csv.reader(io.StringIO(encoded.decode("utf-8-sig")))), content)

    def test_reject_changed_partner_center_field_identity(self):
        rows = copy.deepcopy(self.rows)
        rows[1][1] = "999999"
        with self.assertRaisesRegex(ValueError, "Field/ID/Type changed"):
            store.validate_rows(rows, self.template, store.SOURCE / "assets")

    def test_reject_image_path_traversal_and_missing_root(self):
        for path in ["/tmp/logo.png", "images/art/logo.png",
                     "wirecraft-store/images/../private.png",
                     "wirecraft-store\\images\\logo.png", "https://example.com/a.png"]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                store.safe_image_path(path)

    def test_reject_missing_image(self):
        rows = copy.deepcopy(self.rows)
        next(row for row in rows[1:] if row[0] == "DesktopScreenshot1")[3] = (
            "wirecraft-store/images/screenshots/missing.png"
        )
        with self.assertRaisesRegex(ValueError, "Missing image"):
            store.validate_rows(rows, self.template, store.SOURCE / "assets")

    def test_reject_oversize_translation_and_count_utf16(self):
        data = copy.deepcopy(self.translations["en"])
        data["features"][0] = "😀" * 101
        with self.assertRaisesRegex(ValueError, "oversized features"):
            store.validate_locale("en", data)

    def test_reject_incomplete_locale_and_keyword_spam(self):
        for field, value in [("captions", ["Only one"]),
                             ("searchTerms", ["many words " * 2] * 7)]:
            data = copy.deepcopy(self.translations["en"])
            data[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                store.validate_locale("en", data)

    def test_english_subset_keeps_original_locale_names(self):
        rows = store.make_rows(self.template, self.translations, self.manifest,
                               ["en"], self.identity)
        self.assertEqual(rows[0], self.template[0][:4] + ["en"])
        store.validate_rows(rows, self.template, store.SOURCE / "assets")

    def test_import_is_self_contained_after_root_is_moved(self):
        # Recreate the import root elsewhere: no paths may rely on repository cwd.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / store.BUNDLE
            for row in self.rows[1:]:
                if row[0] and row[2].startswith("Relative path"):
                    for value in set(row[3:]):
                        if value:
                            relative = store.safe_image_path(value)
                            target = root / "images" / relative
                            target.parent.mkdir(parents=True, exist_ok=True)
                            if not target.exists():
                                target.write_bytes((store.SOURCE / "assets" / relative).read_bytes())
            store.validate_rows(self.rows, self.template, root / "images")

    def test_no_fabricated_trailers_other_devices_or_initial_release_notes(self):
        for row in self.rows[1:]:
            if row[0].startswith(("Trailer", "Mobile", "Xbox", "Holographic", "SurfaceHub")) \
                    or row[0] in ["ReleaseNotes", "PromoImage2400x1200"]:
                self.assertFalse(any(row[3:]), row[0])

    def test_every_screenshot_has_a_saved_scene_and_receipt(self):
        receipts = json.loads((store.SOURCE / "captures/capture-receipts.json").read_text())
        self.assertEqual(len(self.manifest["screenshots"]), store.SCREENSHOTS)
        for shot in self.manifest["screenshots"]:
            self.assertTrue((store.ROOT / shot["source"]).is_relative_to(store.SOURCE))
            self.assertTrue((store.ROOT / shot["source"]).is_file())
            receipt = receipts[Path(shot["file"]).stem]
            self.assertEqual(receipt["sha256"], shot["sha256"])
            # The real application at the listing's version, not a mock-up.
            self.assertEqual(receipt["application"], "Wirecraft " + self.manifest["minimumApplicationVersion"])

    def test_artwork_is_composed_from_the_current_screenshots(self):
        sources = json.loads((store.SOURCE / "assets/art/sources.json").read_text())
        for file, digest in sources["screenshots"].items():
            path = store.SOURCE / "assets" / file
            self.assertEqual(store.hashlib.sha256(path.read_bytes()).hexdigest(), digest, file)


if __name__ == "__main__":
    unittest.main()
