import os
from pathlib import Path
import subprocess
import tempfile
import unittest

HELPER = Path(__file__).resolve().parents[2] / "packaging/linux/xkb-runtime.sh"


@unittest.skipIf(os.name == "nt", "POSIX Linux launcher")
class XkbRuntimeTests(unittest.TestCase):
    def fallback(self, paths):
        result = subprocess.run(
            ["sh", "-c", '. "$1"; wirecraft_needs_bundled_xkb "$2"',
             "xkb-test", str(HELPER), ":".join(map(str, paths))],
            capture_output=True, text=True,
        )
        self.assertIn(result.returncode, (0, 1), result.stderr)
        return result.returncode == 0

    def test_missing_bridge_or_entire_runtime_uses_bundled_pair(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.assertTrue(self.fallback([root]))
            (root / "libxkbcommon.so.0").touch()
            self.assertTrue(self.fallback([root]))

    def test_coherent_host_pair_preserves_host_library_selection(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "host with spaces[*]"
            root.mkdir()
            (root / "libxkbcommon.so.0").touch()
            (root / "libxkbcommon-x11.so.0").touch()
            self.assertFalse(self.fallback([root]))

    def test_earlier_core_cannot_be_mixed_with_later_bridge(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            first, second = root / "first", root / "second"
            first.mkdir()
            second.mkdir()
            (first / "libxkbcommon.so.0").touch()
            (second / "libxkbcommon.so.0").touch()
            (second / "libxkbcommon-x11.so.0").touch()
            self.assertTrue(self.fallback([first, second]))
            self.assertFalse(self.fallback([second, first]))

    def test_earlier_bridge_cannot_be_mixed_with_later_core(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            first, second = root / "first", root / "second"
            first.mkdir()
            second.mkdir()
            (first / "libxkbcommon-x11.so.0").touch()
            (second / "libxkbcommon.so.0").touch()
            self.assertTrue(self.fallback([first, second]))


if __name__ == "__main__":
    unittest.main()
