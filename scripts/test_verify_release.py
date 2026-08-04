import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().with_name("verify_release.py")
SPEC = importlib.util.spec_from_file_location("verify_release", SCRIPT)
verify_release = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(verify_release)


class VerifyReleaseTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        root = Path(self.directory.name)
        self.manifest = root / "Cargo.toml"
        self.lockfile = root / "Cargo.lock"
        self.manifest.write_text(
            '[package]\nname = "vibi-dpu"\nversion = "2.2.2"\n'
        )
        self.lockfile.write_text(
            'version = 3\n\n[[package]]\nname = "vibi-dpu"\nversion = "2.2.2"\n'
        )

    def tearDown(self):
        self.directory.cleanup()

    def test_accepts_matching_tag_manifest_and_lockfile(self):
        self.assertEqual(
            verify_release.verify("v2.2.2", self.manifest, self.lockfile), "2.2.2"
        )

    def test_rejects_tag_mismatch(self):
        with self.assertRaisesRegex(ValueError, "does not match Cargo.toml"):
            verify_release.verify("v2.2.3", self.manifest, self.lockfile)

    def test_rejects_lockfile_mismatch(self):
        self.lockfile.write_text(
            'version = 3\n\n[[package]]\nname = "vibi-dpu"\nversion = "2.2.1"\n'
        )
        with self.assertRaisesRegex(ValueError, "exactly one vibi-dpu entry"):
            verify_release.verify("v2.2.2", self.manifest, self.lockfile)

    def test_rejects_non_version_tag(self):
        with self.assertRaisesRegex(ValueError, "vMAJOR.MINOR.PATCH"):
            verify_release.verify("latest", self.manifest, self.lockfile)


if __name__ == "__main__":
    unittest.main()
