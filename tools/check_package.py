#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Test the shipped crate and one frozen downstream consumer, without publishing.

Requires Python 3.9+, Cargo/rustup, Git, and the local v0.1.0 tag (CI fetches tags).
Run: python3 tools/check_package.py --toolchain 1.99.1
For an intentionally uncommitted development candidate, add --allow-dirty.
All builds use offline dependency resolution: the runtime must remain dependency-free.
Temporary extraction and downstream builds are isolated from the source checkout.
"""
import argparse
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(command, cwd, capture=False, env=None):
    print("+ " + " ".join(map(str, command)), flush=True)
    return subprocess.run(command, cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None, env=env)


def extract(archive, destination):
    # Only ordinary files/directories from our own Cargo/Git archives are needed.
    # Reject links and traversal rather than relying on Python-version defaults.
    destination.mkdir(parents=True)
    root = destination.resolve()
    for member in archive.getmembers():
        path = (root / member.name).resolve()
        if root not in path.parents or not (member.isfile() or member.isdir()):
            raise ValueError("unsafe or unsupported archive member: " + member.name)
    archive.extractall(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="1.99.1")
    parser.add_argument("--allow-dirty", action="store_true")
    args = parser.parse_args()
    cargo = ["cargo", "+" + args.toolchain]
    metadata = json.loads(run(cargo + ["metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"], ROOT, True).stdout)
    package = next(p for p in metadata["packages"] if p["name"] == "rcct")
    if package["dependencies"]:
        raise SystemExit("rcct is required to remain dependency-free")
    command = cargo + ["package", "--locked", "--offline"]
    if args.allow_dirty:
        command.append("--allow-dirty")
    run(command, ROOT)
    archive_path = Path(metadata["target_directory"]) / "package" / ("rcct-" + package["version"] + ".crate")
    with tempfile.TemporaryDirectory(prefix="rcct-package-check-") as temporary:
        temporary = Path(temporary)
        with tarfile.open(archive_path, "r:gz") as archive:
            extract(archive, temporary / "package")
        packaged = temporary / "package" / ("rcct-" + package["version"])
        baseline_bytes = subprocess.check_output(["git", "archive", "--format=tar", "v0.1.0"], cwd=ROOT)
        with tarfile.open(fileobj=io.BytesIO(baseline_bytes)) as archive:
            extract(archive, temporary / "baseline")
        baseline = temporary / "baseline"
        source = ROOT / "tests" / "api_compat.rs"
        for relative in ("LICENSE", "NOTICE.md", "README.md", "docs/numerics.md",
                         "tests/api_compat.rs", "tools/generate_numeric_fixtures.py",
                         "tools/generate_orientation_fixtures.py",
                         "tools/generate_crossing_order_fixtures.py",
                         "tools/generate_parameter_enclosures.py",
                         "tools/generate_crossing_coordinate_enclosures.py",
                         "tools/generate_crossing_point_order.py",
                         "tools/generate_constructed_orientation.py"):
            if (packaged / relative).read_bytes() != (ROOT / relative).read_bytes():
                raise SystemExit("packaged file differs from checkout: " + relative)
        # Override any inherited target directory so packaged/downstream evidence
        # cannot silently come from the source checkout's build artifacts.
        env = dict(os.environ, CARGO_TARGET_DIR=str(temporary / "target"))
        for release in (False, True):
            profile = ["--release"] if release else []
            run(cargo + ["test", "--locked", "--offline", "--all-features"] + profile, packaged, env=env)
        for label, dependency in (("baseline", baseline), ("candidate", packaged)):
            consumer = temporary / ("consumer-" + label)
            (consumer / "tests").mkdir(parents=True)
            (consumer / "src").mkdir()
            (consumer / "src" / "lib.rs").write_text("// Separate downstream consumer.\n", encoding="utf-8")
            manifest = ('[package]\nname = "rcct-consumer-' + label + '"\nversion = "0.0.0"\nedition = "2024"\n'
                        '[dependencies]\nrcct = { path = ' + json.dumps(dependency.as_posix()) + ' }\n')
            (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8")
            shutil.copyfile(source, consumer / "tests" / "api_compat.rs")
            run(cargo + ["generate-lockfile", "--offline"], consumer, env=env)
            for release in (False, True):
                run(cargo + ["test", "--locked", "--offline"] + (["--release"] if release else []), consumer, env=env)
    print("PASS: packaged crate and identical v0.1.0/candidate downstream source, debug and release")


if __name__ == "__main__":
    main()
