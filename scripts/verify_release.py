#!/usr/bin/env python3
"""Validate that a release tag matches Cargo.toml and Cargo.lock."""

import argparse
from pathlib import Path
import re
import tomllib

SEMVER_TAG = re.compile(r"^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def verify(tag: str, manifest_path: Path, lockfile_path: Path) -> str:
    match = SEMVER_TAG.fullmatch(tag)
    if match is None:
        raise ValueError(f"release tag must be vMAJOR.MINOR.PATCH, got {tag!r}")
    tag_version = ".".join(match.groups())

    with manifest_path.open("rb") as stream:
        manifest_version = tomllib.load(stream)["package"]["version"]
    with lockfile_path.open("rb") as stream:
        lockfile = tomllib.load(stream)

    locked_versions = [
        package["version"]
        for package in lockfile.get("package", [])
        if package.get("name") == "vibi-dpu"
    ]
    if locked_versions != [manifest_version]:
        raise ValueError(
            "Cargo.lock must contain exactly one vibi-dpu entry matching Cargo.toml; "
            f"manifest={manifest_version!r}, lock={locked_versions!r}"
        )
    if tag_version != manifest_version:
        raise ValueError(
            f"tag version {tag_version!r} does not match Cargo.toml {manifest_version!r}"
        )
    return manifest_version


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument(
        "--manifest", type=Path, default=Path("vibi-dpu/Cargo.toml")
    )
    parser.add_argument(
        "--lockfile", type=Path, default=Path("vibi-dpu/Cargo.lock")
    )
    args = parser.parse_args()
    print(verify(args.tag, args.manifest, args.lockfile))


if __name__ == "__main__":
    main()
