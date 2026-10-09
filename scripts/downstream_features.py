#!/usr/bin/env python3

"""Select optional downstream checks supported by a checked-out manifest."""

import argparse
from pathlib import Path
import tomllib


def select_declared_features(
    manifest_path: Path, package_name: str, requested_features: list[str]
) -> list[str]:
    """Return requested feature names declared by the named package."""
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    package = manifest.get("package", {}).get("name")
    if package != package_name:
        raise ValueError(f"expected package {package_name}, found {package}")
    declared = manifest.get("features", {})
    return [feature for feature in requested_features if feature in declared]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("package")
    parser.add_argument("features", nargs="+")
    arguments = parser.parse_args()
    for feature in select_declared_features(
        arguments.manifest, arguments.package, arguments.features
    ):
        print(feature)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
