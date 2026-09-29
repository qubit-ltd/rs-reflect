#!/usr/bin/env python3
"""Compile and execute isolated Rust Markdown examples with a checked lockfile."""

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tempfile
import tomllib


class ExampleError(RuntimeError):
    """A source-located parsing, compilation, or execution failure."""


@dataclass(frozen=True)
class Block:
    document: Path
    line: int
    mode: str
    source: str
    source_reference: str | None = None

    @property
    def location(self):
        return f"{self.document}:{self.line}"


def read_lines(path):
    """Normalize CRLF only; keep Unicode separators and literal carriage returns."""
    with path.open(encoding="utf-8", newline="") as source:
        text = source.read().replace("\r\n", "\n")
    return text.removesuffix("\n").split("\n")


def source_region(root, reference, location):
    """Read the single complete displayed program without allowing path escape."""
    path = Path(reference)
    label = f"{location}: source {reference}"
    if path.is_absolute() or ".." in path.parts:
        raise ExampleError(f"{label}: invalid source path")
    resolved = (root / path).resolve()
    if not resolved.is_relative_to(root.resolve()):
        raise ExampleError(f"{label}: source escapes repository")
    try:
        lines = read_lines(resolved)
    except OSError as error:
        raise ExampleError(f"{label}: cannot read source: {error}") from error
    starts = [index for index, line in enumerate(lines) if line == "// reflect-example-start"]
    ends = [index for index, line in enumerate(lines) if line == "// reflect-example-end"]
    if len(starts) != 1 or len(ends) != 1 or starts[0] >= ends[0]:
        raise ExampleError(f"{label}: expected one closed source region")
    # Only a leading license header and blank lines may sit outside the program.
    prefix = "\n".join(lines[:starts[0]]).strip()
    license_line = re.compile(r"//(?:\s*(?:SPDX-License-Identifier:|Copyright\b|Licensed under\b).*|\s*[=-]+\s*|\s*)")
    if prefix and not all(not line.strip() or license_line.fullmatch(line) for line in lines[:starts[0]]):
        raise ExampleError(f"{label}: code outside source region")
    if any(line.strip() for line in lines[ends[0] + 1:]):
        raise ExampleError(f"{label}: code outside source region")
    return "\n".join(lines[starts[0] + 1:ends[0]]) + "\n"


def parse_document(document, root=None):
    """Read fenced Rust programs; reject unsupported or incomplete examples."""
    lines = read_lines(document)
    root = Path(root) if root is not None else Path(__file__).resolve().parents[1]
    blocks = []
    fence = None
    pending = None
    for index, line in enumerate(lines, 1):
        match = re.match(r"^\s{0,3}(`{3,}|~{3,})(.*)$", line)
        if fence is None:
            reference = re.fullmatch(r"\s*<!-- reflect-source:\s*(\S+)\s*-->\s*", line)
            if reference:
                if pending:
                    raise ExampleError(f"{document}:{pending[1]}: source {pending[0]}: dangling source marker")
                pending = (reference[1], index)
                continue
            if pending and line.strip() and not match:
                raise ExampleError(f"{document}:{pending[1]}: source {pending[0]}: marker requires adjacent Rust fence")
            if not match:
                continue
            marker, info = match.groups()
            info = info.strip()
            rust = re.match(r"^rust(?:\s|,|$)", info) is not None
            mode = None
            source_reference = pending[0] if pending else None
            if pending and (not rust or info != "rust"):
                raise ExampleError(f"{document}:{index}: source {pending[0]}: source marker requires run Rust fence")
            if rust:
                parts = [part.strip() for part in info.split(",")]
                modes = {("rust",): "run", ("rust", "no_run"): "no_run", ("rust", "compile_fail"): "compile_fail"}
                mode = modes.get(tuple(parts))
                if mode is None:
                    raise ExampleError(f"{document}:{index}: unsupported Rust fence {info!r}")
                if mode != "run":
                    previous = next((text.strip() for text in reversed(lines[:index - 1]) if text.strip()), "")
                    if not previous or previous.startswith(("```", "~~~", "#")):
                        raise ExampleError(f"{document}:{index}: {mode} requires a preceding explanation")
            fence = (marker, index, mode, [], source_reference)
            pending = None
        else:
            marker, start, mode, source, source_reference = fence
            if match and match[1][0] == marker[0] and len(match[1]) >= len(marker) and not match[2].strip():
                if mode:
                    if not any(text.strip() for text in source):
                        reference_label = f"source {source_reference}: " if source_reference else ""
                        raise ExampleError(f"{document}:{start}: {reference_label}empty Rust block")
                    program = "\n".join(source) + "\n"
                    if source_reference and program != source_region(root, source_reference, f"{document}:{start}"):
                        raise ExampleError(f"{document}:{start}: source {source_reference}: Markdown/source drift")
                    blocks.append(Block(document, start, mode, program, source_reference))
                fence = None
            else:
                source.append(line)
    if pending:
        raise ExampleError(f"{document}:{pending[1]}: source {pending[0]}: dangling source marker")
    if fence is not None:
        reference_label = f"source {fence[4]}: " if fence[4] else ""
        raise ExampleError(f"{document}:{fence[1]}: {reference_label}unclosed fence")
    if not blocks:
        raise ExampleError(f"{document}:1: no Rust blocks found")
    return blocks


def verify_lock(baseline, resolved):
    """For shared dependency names/sources, reject newly selected versions."""
    original = tomllib.loads(baseline.read_text(encoding="utf-8"))
    current = tomllib.loads(resolved.read_text(encoding="utf-8"))
    versions = {}
    for package in original.get("package", []):
        versions.setdefault(package["name"], set()).add((package["version"], package.get("source")))
    for package in current.get("package", []):
        if package["name"] in versions and (package["version"], package.get("source")) not in versions[package["name"]]:
            raise ExampleError(f"dependency lock drift: {package['name']} {package['version']} {package.get('source')}")


def command(args, workspace, label, timeout=None, env=None):
    """Record a command and reap its process group, including on timeout."""
    log = workspace / "commands.jsonl"
    with subprocess.Popen(args, cwd=workspace, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, start_new_session=True) as process:
        timed_out = False
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
        with log.open("a", encoding="utf-8") as output:
            output.write(json.dumps({"label": label, "command": args, "pid": process.pid, "exit_code": process.returncode,
                                     "timeout": timed_out, "stdout": stdout, "stderr": stderr}) + "\n")
        if timed_out:
            raise ExampleError(f"{label}: timeout after {timeout} seconds")
        return process.returncode, stdout, stderr


def check_examples(root, documents, timeout=10, dependency_name="qubit-reflect"):
    """Run each block in its own package; retain owned evidence on failure."""
    blocks = [block for document in documents for block in parse_document(document, root)]
    workspace = Path(tempfile.mkdtemp(prefix="rs-reflect-markdown."))
    success = False
    try:
        members = []
        # The independent probe prevents dependency failures from satisfying compile_fail.
        probe = Block(root / "Cargo.toml", 1, "no_run", "")
        for index, block in enumerate([probe, *blocks]):
            name = f"markdown-example-{index}"
            members.append(name)
            package = workspace / name
            (package / "src").mkdir(parents=True)
            dependencies = f'{json.dumps(dependency_name)}={{path={json.dumps(str(root))}}}\n'
            if block.document.resolve().parent == (root / "derive").resolve():
                dependencies += f'"qubit-reflect-derive"={{path={json.dumps(str(root / "derive"))}}}\n'
            manifest = (
                f'[package]\nname={json.dumps(name)}\nversion="0.0.0"\nedition="2024"\npublish=false\n'
                f'[dependencies]\n{dependencies}'
            )
            (package / "Cargo.toml").write_text(manifest, encoding="utf-8")
            (package / "src" / ("main.rs" if block.mode == "run" else "lib.rs")).write_text(block.source, encoding="utf-8")
        (workspace / "Cargo.toml").write_text(f'[workspace]\nresolver="3"\nmembers={json.dumps(members)}\n', encoding="utf-8")
        shutil.copyfile(root / "Cargo.lock", workspace / "Cargo.lock")
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(Path(env.get("CARGO_TARGET_DIR", root / "target/markdown-examples")).resolve())
        for args in [["rustc", "--version", "--verbose"], ["cargo", "--version"],
                     ["cargo", "metadata", "--offline", "--format-version=1"]]:
            code, _, stderr = command(args, workspace, "dependency resolution", env=env)
            if code:
                raise ExampleError(f"dependency resolution failed: {stderr}")
        verify_lock(root / "Cargo.lock", workspace / "Cargo.lock")
        (workspace / "examples.json").write_text(json.dumps([
            {"package": members[index], "document": str(block.document), "line": block.line, "mode": block.mode}
            for index, block in enumerate([probe, *blocks])], indent=2), encoding="utf-8")
        for index, block in enumerate([probe, *blocks]):
            package = members[index]
            label = f"{block.location} [{package}]"
            args = ["cargo", "build", "--offline", "--locked", "--package", package, "--message-format=json"]
            code, stdout, stderr = command(args, workspace, f"{label} compile", env=env)
            if block.mode == "compile_fail":
                diagnostics = [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]
                own_error = any(message.get("reason") == "compiler-message"
                                and message.get("target", {}).get("name") == package.replace("-", "_")
                                and message.get("message", {}).get("level") == "error" for message in diagnostics)
                if code == 0:
                    raise ExampleError(f"{label}: compile_fail compiled successfully")
                if not own_error:
                    raise ExampleError(f"{label}: compile failed outside the example: {stderr}")
            elif code:
                messages = [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]
                diagnostics = "".join(item.get("message", {}).get("rendered", "") or ""
                                      for item in messages if item.get("reason") == "compiler-message")
                raise ExampleError(f"{label}: compile failed: {stderr}\n{diagnostics}")
            elif block.mode == "run":
                artifacts = [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]
                executable = next((item.get("executable") for item in artifacts
                                   if item.get("reason") == "compiler-artifact" and item.get("executable")), None)
                if not executable:
                    raise ExampleError(f"{label}: compile produced no executable")
                code, stdout, stderr = command([executable], workspace, f"{label} run", timeout, env)
                if code:
                    raise ExampleError(f"{label}: run exited {code}: {stdout}{stderr}")
            print(f"OK {label}: {block.mode}")
        success = True
    except Exception as error:
        raise ExampleError(f"{error}\nEvidence retained at {workspace}") from error
    finally:
        if success:
            shutil.rmtree(workspace)
    return len(blocks)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("documents", nargs="*", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    documents = args.documents or [root / name for name in [
        "README.md",
        "README.zh_CN.md",
        "derive/README.md",
        "derive/README.zh_CN.md",
        "doc/user_guide.md",
        "doc/user_guide.zh_CN.md",
    ]]
    try:
        count = check_examples(root, documents)
    except ExampleError as error:
        parser.exit(1, f"{error}\n")
    print(f"Validated {count} isolated Rust Markdown examples.")


if __name__ == "__main__":
    main()
