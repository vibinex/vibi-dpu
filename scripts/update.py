#!/usr/bin/env python3
"""Increment the vibi-dpu patch version in Cargo.toml and Cargo.lock."""

from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "vibi-dpu" / "Cargo.toml"
LOCKFILE = ROOT / "vibi-dpu" / "Cargo.lock"
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def package_version(document: str) -> str:
    package = re.search(r"(?ms)^\[package\]\s*(.*?)(?=^\[|\Z)", document)
    if package is None:
        raise ValueError("Cargo.toml has no [package] table")
    version = re.search(r'^version\s*=\s*"([^"]+)"\s*$', package.group(1), re.MULTILINE)
    if version is None:
        raise ValueError("Cargo.toml [package] has no version")
    return version.group(1)


def next_patch(version: str) -> str:
    match = SEMVER.fullmatch(version)
    if match is None:
        raise ValueError(f"expected a stable MAJOR.MINOR.PATCH version, got {version!r}")
    major, minor, patch = (int(part) for part in match.groups())
    return f"{major}.{minor}.{patch + 1}"


def next_untagged_patch(version: str, tags: set[str]) -> str:
    candidate = next_patch(version)
    while f"v{candidate}" in tags or candidate in tags:
        candidate = next_patch(candidate)
    return candidate


def repository_tags() -> set[str]:
    result = subprocess.run(
        ["git", "tag", "--list"], cwd=ROOT, check=True, text=True, capture_output=True
    )
    return set(result.stdout.splitlines())


def replace_manifest_version(document: str, old: str, new: str) -> str:
    package = re.search(r"(?ms)^\[package\]\s*(.*?)(?=^\[|\Z)", document)
    assert package is not None
    updated, count = re.subn(
        rf'(?m)^version\s*=\s*"{re.escape(old)}"\s*$',
        f'version = "{new}"',
        package.group(0),
        count=1,
    )
    if count != 1:
        raise ValueError("could not update Cargo.toml [package] version")
    return document[: package.start()] + updated + document[package.end() :]


def replace_lock_version(document: str, old: str, new: str) -> str:
    package = re.search(
        r'(?ms)^\[\[package\]\]\s*\nname = "vibi-dpu"\s*\nversion = "([^"]+)"(.*?)(?=^\[\[package\]\]|\Z)',
        document,
    )
    if package is None:
        raise ValueError("Cargo.lock has no vibi-dpu package entry")
    if package.group(1) != old:
        raise ValueError(
            f"Cargo.toml version {old} does not match Cargo.lock vibi-dpu version {package.group(1)}"
        )
    updated = package.group(0).replace(f'version = "{old}"', f'version = "{new}"', 1)
    return document[: package.start()] + updated + document[package.end() :]


def main(tags: set[str] | None = None) -> None:
    manifest = MANIFEST.read_text()
    lockfile = LOCKFILE.read_text()
    old = package_version(manifest)
    new = next_untagged_patch(old, repository_tags() if tags is None else tags)

    # Validate both documents before writing either one.
    updated_manifest = replace_manifest_version(manifest, old, new)
    updated_lockfile = replace_lock_version(lockfile, old, new)
    MANIFEST.write_text(updated_manifest)
    LOCKFILE.write_text(updated_lockfile)
    print(f"Updated vibi-dpu from {old} to {new}")


if __name__ == "__main__":
    main()
