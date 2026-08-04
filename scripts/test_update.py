import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().with_name("update.py")
SPEC = importlib.util.spec_from_file_location("update", SCRIPT)
update = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(update)


class UpdateVersionTest(unittest.TestCase):
    def test_increments_manifest_and_lockfile_together(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "Cargo.toml"
            lockfile = root / "Cargo.lock"
            manifest.write_text(
                '[package]\nname = "vibi-dpu"\nversion = "1.2.3"\nedition = "2021"\n'
            )
            lockfile.write_text(
                'version = 3\n\n[[package]]\nname = "vibi-dpu"\nversion = "1.2.3"\n'
            )

            with patch.object(update, "MANIFEST", manifest), patch.object(
                update, "LOCKFILE", lockfile
            ):
                update.main(tags=set())

            self.assertIn('version = "1.2.4"', manifest.read_text())
            self.assertIn('version = "1.2.4"', lockfile.read_text())

    def test_refuses_mismatched_lockfile_without_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "Cargo.toml"
            lockfile = root / "Cargo.lock"
            manifest_text = '[package]\nname = "vibi-dpu"\nversion = "1.2.3"\n'
            lockfile_text = (
                'version = 3\n\n[[package]]\nname = "vibi-dpu"\nversion = "1.2.2"\n'
            )
            manifest.write_text(manifest_text)
            lockfile.write_text(lockfile_text)

            with patch.object(update, "MANIFEST", manifest), patch.object(
                update, "LOCKFILE", lockfile
            ), self.assertRaisesRegex(ValueError, "does not match"):
                update.main(tags=set())

            self.assertEqual(manifest.read_text(), manifest_text)
            self.assertEqual(lockfile.read_text(), lockfile_text)

    def test_skips_versions_that_already_have_release_tags(self):
        for existing_tag in ("2.2.1", "v2.2.1"):
            with self.subTest(existing_tag=existing_tag):
                self.assertEqual(
                    update.next_untagged_patch("2.2.0", {existing_tag}), "2.2.2"
                )

    def test_rejects_non_stable_version(self):
        with self.assertRaisesRegex(ValueError, "stable MAJOR.MINOR.PATCH"):
            update.next_patch("1.2.3-rc.1")


if __name__ == "__main__":
    unittest.main()
