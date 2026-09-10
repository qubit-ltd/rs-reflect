#!/usr/bin/env python3
"""Validate and inspect the fixed downstream repository layout."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys


SCHEMA_VERSION = 1
REPOSITORIES = (
    ("qubit-ltd/rs-model-metadata", Path("rust-platform/rs-model-metadata")),
    ("qubit-ltd/rs-platform", Path("rust-platform/rs-platform")),
    ("qubit-ltd/rs-id", Path("rust-common/rs-id")),
    ("qubit-ltd/rs-datatype", Path("rust-common/rs-datatype")),
    ("qubit-ltd/rs-redact", Path("rust-common/rs-redact")),
    ("qubit-ltd/rs-validator", Path("rust-common/rs-validator")),
    ("qubit-ltd/rs-validation-rules", Path("rust-common/rs-validation-rules")),
)
EXPECTED_BY_REPOSITORY = dict(REPOSITORIES)
SHA_PATTERN = re.compile(r"[0-9a-fA-F]{40}")


class ManifestError(Exception):
    """A safe, user-facing manifest or checkout error."""


def _read_manifest(path):
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ManifestError(f"cannot read manifest {Path(path)}: {error}") from None
    return _validate_manifest(value)


def _validate_manifest(value):
    if not isinstance(value, dict):
        raise ManifestError("manifest: expected an object")
    unknown = set(value) - {"schema_version", "repositories"}
    if unknown:
        raise ManifestError(f"manifest: unknown field {sorted(unknown)[0]}")
    if set(value) != {"schema_version", "repositories"}:
        missing = sorted({"schema_version", "repositories"} - set(value))[0]
        raise ManifestError(f"manifest: missing field {missing}")
    if type(value["schema_version"]) is not int or value["schema_version"] != SCHEMA_VERSION:
        raise ManifestError("manifest.schema_version: expected 1")
    entries = value["repositories"]
    if not isinstance(entries, list):
        raise ManifestError("manifest.repositories: expected an array")

    validated = {}
    seen_paths = set()
    for index, entry in enumerate(entries):
        field = f"manifest.repositories[{index}]"
        if not isinstance(entry, dict):
            raise ManifestError(f"{field}: expected an object")
        unknown = set(entry) - {"repository", "path", "revision"}
        if unknown:
            raise ManifestError(f"{field}: unknown field {sorted(unknown)[0]}")
        missing = {"repository", "path", "revision"} - set(entry)
        if missing:
            raise ManifestError(f"{field}: missing field {sorted(missing)[0]}")

        repository = entry["repository"]
        path_text = entry["path"]
        revision = entry["revision"]
        if not isinstance(repository, str) or repository not in EXPECTED_BY_REPOSITORY:
            raise ManifestError(f"{field}.repository: unknown repository")
        if repository in validated:
            raise ManifestError(f"{field}.repository: duplicate repository {repository}")
        if not isinstance(path_text, str):
            raise ManifestError(f"{field}.path: expected a string")
        candidate = Path(path_text)
        if candidate.is_absolute() or ".." in candidate.parts:
            raise ManifestError(f"{field}.path: unsafe path")
        expected_path = EXPECTED_BY_REPOSITORY[repository]
        if candidate != expected_path:
            raise ManifestError(f"{field}.path: path does not match repository {repository}")
        if candidate in seen_paths:
            raise ManifestError(f"{field}.path: duplicate path {path_text}")
        if not isinstance(revision, str) or SHA_PATTERN.fullmatch(revision) is None:
            raise ManifestError(f"{field}.revision: expected a full 40-character Git SHA")

        seen_paths.add(candidate)
        validated[repository] = {
            "repository": repository,
            "path": candidate.as_posix(),
            "revision": revision,
        }

    missing_repositories = [
        repository for repository, _ in REPOSITORIES if repository not in validated
    ]
    if missing_repositories:
        raise ManifestError(f"manifest.repositories: missing repository {missing_repositories[0]}")
    if len(validated) != len(REPOSITORIES):
        raise ManifestError(
            f"manifest.repositories: expected exactly {len(REPOSITORIES)} repositories"
        )
    return [validated[repository] for repository, _ in REPOSITORIES]


def _git(layout_root, repository, relative_path, arguments):
    root = Path(layout_root).resolve()
    checkout = (root / relative_path).resolve()
    try:
        checkout.relative_to(root)
    except ValueError:
        raise ManifestError(f"{repository}: checkout path escapes layout root") from None
    command = ["git", "-C", str(checkout), *arguments]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        operation = "git " + " ".join(arguments)
        raise ManifestError(f"{repository}: {operation} failed")
    return result.stdout


def _inspect_checkout(layout_root, repository, relative_path):
    revision = _git(layout_root, repository, relative_path, ["rev-parse", "HEAD"]).strip()
    if SHA_PATTERN.fullmatch(revision) is None:
        raise ManifestError(f"{repository}: git rev-parse returned an invalid HEAD")
    status = _git(layout_root, repository, relative_path, ["status", "--porcelain"])
    if status:
        raise ManifestError(f"{repository}: checkout is dirty")
    return revision.lower()


def _matrix(entries, mode):
    revision_by_repository = (
        {entry["repository"]: entry["revision"] for entry in entries}
        if entries is not None
        else {}
    )
    return {
        "repositories": {
            _repository_key(repository): {
                "repository": repository,
                "path": relative_path.as_posix(),
                "revision": (
                    revision_by_repository[repository] if mode == "baseline" else "main"
                ),
            }
            for repository, relative_path in REPOSITORIES
        }
    }


def _repository_key(repository):
    """Return a stable expression-safe key for a GitHub Actions output."""
    return repository.rsplit("/", 1)[-1].replace("-", "_")


def _collect(layout_root):
    entries = []
    for repository, relative_path in REPOSITORIES:
        entries.append(
            {
                "repository": repository,
                "path": relative_path.as_posix(),
                "revision": _inspect_checkout(layout_root, repository, relative_path),
            }
        )
    return {"schema_version": SCHEMA_VERSION, "repositories": entries}


def _write_manifest(path, manifest):
    output = Path(path)
    output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def _verify(layout_root, entries):
    errors = []
    for entry in entries:
        repository = entry["repository"]
        relative_path = EXPECTED_BY_REPOSITORY[repository]
        try:
            actual = _inspect_checkout(layout_root, repository, relative_path)
        except ManifestError as error:
            errors.append(str(error))
            continue
        if actual != entry["revision"].lower():
            errors.append(
                f"{repository}: HEAD mismatch for revision {entry['revision']}"
            )
    if errors:
        raise ManifestError("; ".join(errors))


def _parser():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser("validate")
    validate.add_argument("--manifest", required=True, type=Path)

    matrix = subparsers.add_parser("matrix")
    matrix.add_argument("--manifest", required=True, type=Path)
    matrix.add_argument("--mode", choices=("baseline", "head"), required=True)

    collect = subparsers.add_parser("collect")
    collect.add_argument("--layout-root", required=True, type=Path)
    collect.add_argument("--output", required=True, type=Path)

    verify = subparsers.add_parser("verify")
    verify.add_argument("--layout-root", required=True, type=Path)
    verify.add_argument("--manifest", required=True, type=Path)
    return parser


def main(arguments=None):
    options = _parser().parse_args(arguments)
    try:
        if options.command == "validate":
            _read_manifest(options.manifest)
        elif options.command == "matrix":
            entries = (
                _read_manifest(options.manifest)
                if options.mode == "baseline"
                else None
            )
            print(json.dumps(_matrix(entries, options.mode), separators=(",", ":")))
        elif options.command == "collect":
            _write_manifest(options.output, _collect(options.layout_root))
        elif options.command == "verify":
            _verify(options.layout_root, _read_manifest(options.manifest))
    except (ManifestError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
