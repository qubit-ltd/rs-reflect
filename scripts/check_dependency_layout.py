#!/usr/bin/env python3
"""Check that every local Cargo path dependency is inside the CI checkout layout."""

import argparse
from pathlib import Path
import sys
import tomllib


def _manifests(root: Path):
    return sorted(root.rglob("Cargo.toml"))


def _path_dependencies(value):
    if not isinstance(value, dict):
        return []
    result = []
    for name, dependency in value.items():
        if isinstance(dependency, dict) and isinstance(dependency.get("path"), str):
            result.append((name, dependency["path"]))
    return result


def check(root: Path, project: Path) -> None:
    root = root.resolve()
    project = project.resolve()
    errors = []
    pending = list(_manifests(project))
    visited = set()
    while pending:
        manifest_path = pending.pop()
        if manifest_path in visited:
            continue
        visited.add(manifest_path)
        try:
            manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{manifest_path}: cannot parse Cargo.toml: {error}")
            continue
        sections = [manifest.get("dependencies", {})]
        for section in ("dev-dependencies", "build-dependencies"):
            sections.append(manifest.get(section, {}))
        target = manifest.get("target", {})
        if isinstance(target, dict):
            for target_config in target.values():
                if isinstance(target_config, dict):
                    sections.extend(
                        target_config.get(section, {})
                        for section in ("dependencies", "dev-dependencies", "build-dependencies")
                    )
        workspace_root = next(
            (candidate for candidate in (manifest_path.parent, *manifest_path.parents)
             if (candidate / "Cargo.toml").is_file()
             and "workspace" in tomllib.loads(
                 (candidate / "Cargo.toml").read_text(encoding="utf-8")
             )),
            None,
        )
        if workspace_root is not None:
            workspace_manifest = tomllib.loads(
                (workspace_root / "Cargo.toml").read_text(encoding="utf-8")
            )
            sections.append(
                workspace_manifest.get("workspace", {}).get("dependencies", {})
            )
            for member in workspace_manifest.get("workspace", {}).get("members", []):
                if not isinstance(member, str):
                    continue
                member_root = (workspace_root / member).resolve()
                try:
                    member_root.relative_to(root)
                except ValueError:
                    errors.append(
                        f"{workspace_root / 'Cargo.toml'}: workspace member path "
                        f"{member!r} escapes the CI layout"
                    )
                if member_root.is_relative_to(root) and not (member_root / "Cargo.toml").is_file():
                    errors.append(
                        f"{workspace_root / 'Cargo.toml'}: workspace member path "
                        f"{member!r} is missing at {member_root / 'Cargo.toml'}"
                    )
        for section in sections:
            for dependency, relative in _path_dependencies(section):
                resolved = (manifest_path.parent / relative).resolve()
                try:
                    resolved.relative_to(root)
                except ValueError:
                    errors.append(
                        f"{manifest_path}: dependency {dependency!r} path {relative!r} "
                        "escapes the CI layout"
                    )
                    continue
                if not (resolved / "Cargo.toml").is_file():
                    errors.append(
                        f"{manifest_path}: dependency {dependency!r} path {relative!r} "
                        f"is missing at {resolved / 'Cargo.toml'}"
                    )
                elif resolved / "Cargo.toml" not in visited:
                    pending.append(resolved / "Cargo.toml")
    if errors:
        raise SystemExit("\n".join(f"error: {error}" for error in errors))
    print(f"Cargo path dependency layout is complete for {project}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--layout-root", required=True, type=Path)
    parser.add_argument("--project", required=True, type=Path)
    options = parser.parse_args(argv)
    check(options.layout_root, options.project)


if __name__ == "__main__":
    main()
