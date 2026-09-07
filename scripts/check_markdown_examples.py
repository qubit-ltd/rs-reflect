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

    @property
    def location(self):
        return f"{self.document}:{self.line}"


def parse_document(document):
    """Read fenced Rust programs; reject unsupported or incomplete examples."""
    lines = document.read_text(encoding="utf-8").splitlines()
    blocks = []
    fence = None
    for index, line in enumerate(lines, 1):
        match = re.match(r"^\s{0,3}(`{3,}|~{3,})(.*)$", line)
        if fence is None:
            if not match:
                continue
            marker, info = match.groups()
            info = info.strip()
            rust = re.match(r"^rust(?:\s|,|$)", info) is not None
            mode = None
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
            fence = (marker, index, mode, [])
        else:
            marker, start, mode, source = fence
            if match and match[1][0] == marker[0] and len(match[1]) >= len(marker) and not match[2].strip():
                if mode:
                    if not any(text.strip() for text in source):
                        raise ExampleError(f"{document}:{start}: empty Rust block")
                    blocks.append(Block(document, start, mode, "\n".join(source) + "\n"))
                fence = None
            else:
                source.append(line)
    if fence is not None:
        raise ExampleError(f"{document}:{fence[1]}: unclosed fence")
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
    blocks = [block for document in documents for block in parse_document(document)]
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
            manifest = f'[package]\nname={json.dumps(name)}\nversion="0.0.0"\nedition="2024"\npublish=false\n[dependencies]\n{json.dumps(dependency_name)}={{path={json.dumps(str(root))}}}\n'
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
    documents = args.documents or [root / name for name in ["README.md", "README.zh_CN.md",
        "doc/2026-08-29-qubit-reflect-user-guide.md", "doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md"]]
    try:
        count = check_examples(root, documents)
    except ExampleError as error:
        parser.exit(1, f"{error}\n")
    print(f"Validated {count} isolated Rust Markdown examples.")


if __name__ == "__main__":
    main()
