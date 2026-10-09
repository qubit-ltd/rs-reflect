import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "downstream_features.py"
SPEC = importlib.util.spec_from_file_location("downstream_features", SCRIPT)
downstream_features = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(downstream_features)


class DownstreamFeaturesTests(unittest.TestCase):
    def test_selects_only_features_declared_by_the_checked_out_baseline(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            manifest = Path(temporary_directory) / "Cargo.toml"
            manifest.write_text(
                '[package]\nname = "qubit-model-metadata"\nversion = "0.1.0"\n'
                '[features]\ndefault = []\nvalidation = []\n',
                encoding="utf-8",
            )

            selected = downstream_features.select_declared_features(
                manifest, "qubit-model-metadata", ["generic", "codec", "validation"]
            )

        self.assertEqual(selected, ["validation"])

    def test_rejects_a_manifest_for_a_different_package(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            manifest = Path(temporary_directory) / "Cargo.toml"
            manifest.write_text(
                '[package]\nname = "other-package"\nversion = "0.1.0"\n',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "other-package"):
                downstream_features.select_declared_features(
                    manifest, "qubit-model-metadata", ["generic"]
                )


if __name__ == "__main__":
    unittest.main()
