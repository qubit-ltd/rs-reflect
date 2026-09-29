#!/usr/bin/env python3
"""Verify reflection consumers using packaged sources or crates.io only."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import runpy
from pathlib import Path
import subprocess
import tarfile
import tomllib

PROFILES = {
    "runtime-only": ([], False),
    "default": ([], True),
    "derive-only": (["derive"], False),
    "ecosystem-only": (["ecosystem-types"], False),
    "qubit-only": (["qubit-types"], False),
    "public-all": (["derive", "ecosystem-types", "qubit-types"], False),
}
BASE = '''use qubit_reflect::{Reflect, TypeDescriptor};
fn main() {
    assert!(std::ptr::eq(<u32 as Reflect>::type_descriptor(), TypeDescriptor::of::<u32>()));
    exercise();
}
'''
RUNTIME = "fn exercise() {}\n"
DERIVE = '''#[derive(Reflect)]
struct User { name: String }
fn exercise() {
    use qubit_reflect::{ReflectedMut, ReflectedOwned, ReflectedRef};
    let descriptor = TypeDescriptor::of::<User>();
    let field = descriptor.field("name").expect("derived field");
    let mut user = User { name: String::from("Ada") };
    let value = field.get(ReflectedRef::new(&user)).expect("checked read");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("Ada"));
    field.set(ReflectedMut::new(&mut user), ReflectedOwned::new(String::from("Grace")))
        .expect("checked replacement");
    assert_eq!(user.name, "Grace");
}
'''


def unpack(archive: Path, destination: Path, crate: str, version: str) -> Path:
    """Extract a crate archive without accepting links or path traversal."""
    root_name = f"{crate}-{version}"
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:gz") as packed:
        for member in packed.getmembers():
            parts = Path(member.name).parts
            if not parts or parts[0] != root_name or ".." in parts or Path(member.name).is_absolute():
                raise ValueError(f"invalid archive path: {member.name}")
            target = destination.joinpath(*parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                source = packed.extractfile(member)
                if source is None:
                    raise ValueError(f"unreadable archive member: {member.name}")
                target.write_bytes(source.read())
            else:
                raise ValueError(f"archive links or special files are forbidden: {member.name}")
    result = destination / root_name
    manifest = tomllib.loads((result / "Cargo.toml").read_text())
    if manifest["package"]["name"] != crate or manifest["package"]["version"] != version:
        raise ValueError("archive package identity mismatch")
    return result.resolve()


def validate_sources(metadata: dict, allowed: dict[str, Path], version: str, consumer_manifest: Path) -> None:
    """Require registry provenance outside staged roots and the exact consumer root."""
    root_id = metadata.get("resolve", {}).get("root")
    root_packages = [package for package in metadata["packages"]
                     if root_id is not None and package.get("id") == root_id]
    if len(root_packages) != 1:
        raise ValueError("consumer root package missing or ambiguous")
    consumer = root_packages[0]
    if (Path(consumer["manifest_path"]).resolve() != consumer_manifest.resolve()
            or consumer["source"] is not None or consumer["name"] != "release-consumer"
            or consumer["version"] != "0.0.0"):
        raise ValueError("consumer root identity mismatch")
    seen = set()
    for package in metadata["packages"]:
        name = package["name"]
        if package is consumer:
            continue
        if name in allowed:
            actual = Path(package["manifest_path"]).resolve().parent
            if actual != allowed[name].resolve() or package["source"] is not None:
                raise ValueError(f"unexpected staged source: {name}")
            if package["version"] != version:
                raise ValueError(f"unexpected staged version: {name}")
        elif package["source"] != "registry+https://github.com/rust-lang/crates.io-index":
            raise ValueError(f"non-registry dependency: {name}")
        if name in {"qubit-reflect", "qubit-reflect-derive"}:
            if package["version"] != version:
                raise ValueError(f"reflection version mismatch: {name}")
            seen.add(name)
    if "qubit-reflect" not in seen:
        raise ValueError("runtime package missing")


def fixture(version: str, profile: str, allowed: dict[str, Path]) -> tuple[str, str]:
    """Create a standalone profile using only user-facing reflection features."""
    features, default = PROFILES[profile]
    manifest = ('[package]\nname = "release-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
                '[workspace]\n[dependencies]\n')
    manifest += (f'qubit-reflect = {{ version = "={version}", default-features = '
                 f'{str(default).lower()}, features = {json.dumps(features)} }}\n')
    body = DERIVE if default or "derive" in features else RUNTIME
    if "ecosystem-types" in features:
        manifest += 'bigdecimal = "0.4"\nchrono = { version = "0.4", default-features = false, features = ["std"] }\nuuid = "1.26"\n'
        body = body.replace('fn exercise() {', '''fn exercise() {
    let _ = TypeDescriptor::of::<bigdecimal::BigDecimal>();
    let _ = TypeDescriptor::of::<chrono::NaiveDate>();
    let _ = TypeDescriptor::of::<uuid::Uuid>();''')
    if "qubit-types" in features:
        manifest += 'qubit-id = { version = "0.6.0", default-features = false }\nqubit-datatype = "0.14.0"\n'
        body = body.replace('fn exercise() {', '''fn exercise() {
    let _ = TypeDescriptor::of::<qubit_id::Id>();
    let _ = TypeDescriptor::of::<qubit_datatype::DataType>();''')
    if allowed:
        manifest += '[patch.crates-io]\n'
        for name, path in sorted(allowed.items()):
            manifest += f'{name} = {{ path = {json.dumps(str(path))} }}\n'
    return manifest, BASE + body


PACKAGED_EXAMPLES = ("field_patch", "customer_patch", "support_action")


def packaged_examples_fixture(version: str, allowed: dict[str, Path]) -> str:
    """Use three bin entry points from the staged runtime, never the checkout."""
    if set(allowed) != {"qubit-reflect", "qubit-reflect-derive"}:
        raise ValueError("packaged examples require both staged archive roots")
    runtime = allowed["qubit-reflect"].resolve()
    manifest, _ = fixture(version, "default", allowed)
    for name in PACKAGED_EXAMPLES:
        source = runtime / "examples" / f"{name}.rs"
        if not source.is_file() or not source.resolve().is_relative_to(runtime):
            raise ValueError(f"missing or escaping packaged example: {source}")
        manifest += f'[[bin]]\nname = {json.dumps(name)}\npath = {json.dumps(str(source))}\n'
    return manifest


def check_packaged_examples(root, version, allowed, env, results):
    """Record provenance and fail if any packaged example's main fails."""
    runtime = allowed.get("qubit-reflect")
    if runtime is not None:
        # Check the archived documents against their archived examples before Cargo.
        documents = [runtime / "README.md", runtime / "README.zh_CN.md",
                     runtime / "doc/user_guide.md", runtime / "doc/user_guide.zh_CN.md"]
        linked_documents = [document for document in documents if document.is_file()
                            and "<!-- reflect-source:" in document.read_text(encoding="utf-8")]
        if linked_documents:
            checker = runpy.run_path(str(Path(__file__).with_name("check_markdown_examples.py")))
            for document in linked_documents:
                checker["parse_document"](document, runtime)
    consumer = root / "packaged-examples"
    consumer.mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(packaged_examples_fixture(version, allowed))
    results["packaged_example_sources"] = {
        name: str(allowed["qubit-reflect"].resolve() / "examples" / f"{name}.rs")
        for name in PACKAGED_EXAMPLES
    }
    commands = [
        ["cargo", "+1.94.0", "generate-lockfile"],
        ["cargo", "+1.94.0", "metadata", "--locked", "--format-version", "1"],
        *[["cargo", "+1.94.0", "run", "--locked", "--quiet", "--bin", name] for name in PACKAGED_EXAMPLES],
    ]
    for index, command in enumerate(commands):
        result = subprocess.run(command, cwd=consumer, env=env, capture_output=True, text=True)
        (consumer / f"{index}.stdout.log").write_text(result.stdout)
        (consumer / f"{index}.stderr.log").write_text(result.stderr)
        results["profiles"].setdefault("packaged-examples", []).append({"command": command, "exit_code": result.returncode})
        (root / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"packaged-examples: command failed, see {consumer}")
        if index == 1:
            metadata = json.loads(result.stdout)
            validate_sources(metadata, allowed, version, consumer / "Cargo.toml")
            names = {package["name"] for package in metadata["packages"]}
            if not set(allowed).issubset(names):
                raise ValueError("packaged examples metadata missing staged reflection package")
    print("staged: packaged-examples passed", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', choices=['staged', 'derive-registry', 'registry'], required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--runtime-archive', type=Path)
    parser.add_argument('--derive-archive', type=Path)
    parser.add_argument('--evidence-dir', type=Path, required=True)
    args = parser.parse_args()
    if args.mode == 'staged' and (not args.runtime_archive or not args.derive_archive):
        parser.error('staged mode requires both archives')
    if args.mode == 'derive-registry' and (not args.runtime_archive or args.derive_archive):
        parser.error('derive-registry requires runtime archive only')
    if args.mode == 'registry' and (args.runtime_archive or args.derive_archive):
        parser.error('registry mode forbids archives and local patches')
    root = args.evidence_dir.resolve()
    root.mkdir(parents=True, exist_ok=False)
    allowed = {}
    checksums = {}
    for name, archive in [('qubit-reflect', args.runtime_archive), ('qubit-reflect-derive', args.derive_archive)]:
        if archive:
            archive = archive.resolve()
            checksums[name] = hashlib.sha256(archive.read_bytes()).hexdigest()
            allowed[name] = unpack(archive, root / 'archives', name, args.version)
    env = os.environ.copy()
    env['CARGO_HOME'] = str(root / 'cargo-home')
    env['CARGO_TARGET_DIR'] = str(root / 'target')
    for key in ['RUSTFLAGS', 'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_ENCODED_RUSTDOCFLAGS']:
        env.pop(key, None)
    results = {'mode': args.mode, 'version': args.version, 'archive_sha256': checksums, 'profiles': {}}
    summary = root / 'summary.json'
    for profile in PROFILES:
        consumer = root / profile
        (consumer / 'src').mkdir(parents=True)
        manifest, source = fixture(args.version, profile, allowed)
        (consumer / 'Cargo.toml').write_text(manifest)
        (consumer / 'src/main.rs').write_text(source)
        commands = [
            ['cargo', '+1.94.0', 'generate-lockfile'],
            ['cargo', '+1.94.0', 'metadata', '--locked', '--format-version', '1'],
            ['cargo', '+1.94.0', 'run', '--locked', '--quiet'],
        ]
        for index, command in enumerate(commands):
            result = subprocess.run(command, cwd=consumer, env=env, capture_output=True, text=True)
            (consumer / f'{index}.stdout.log').write_text(result.stdout)
            (consumer / f'{index}.stderr.log').write_text(result.stderr)
            results['profiles'].setdefault(profile, []).append({'command': command, 'exit_code': result.returncode})
            summary.write_text(json.dumps(results, indent=2) + '\n')
            if result.returncode:
                raise RuntimeError(f'{profile}: command failed, see {consumer}')
            if index == 1:
                validate_sources(json.loads(result.stdout), allowed, args.version, consumer / "Cargo.toml")
        print(f'{args.mode}: {profile} passed', flush=True)
    if args.mode == 'staged':
        check_packaged_examples(root, args.version, allowed, env, results)
    results['status'] = 'passed'
    summary.write_text(json.dumps(results, indent=2) + '\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
