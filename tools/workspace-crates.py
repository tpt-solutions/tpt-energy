#!/usr/bin/env python3
"""List the workspace crates that `cargo publish` would upload.

`cargo metadata` is the source of truth: it resolves workspace membership,
optional dependencies, and the `publish` flag, so this cannot drift from what
the release workflow actually does. Parsing the JSON in Python keeps the shell
scripts free of a `jq` dependency.

Usage:
    tools/workspace-crates.py            # one `name<TAB>version<TAB>path` per line
    tools/workspace-crates.py --names    # just the names, one per line
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def publishable_packages() -> list[dict[str, object]]:
    """Every workspace package cargo is willing to publish, name-sorted.

    `cargo metadata` reports `publish` as the list of registries a package may
    be uploaded to: `null` means "crates.io and the source replacement", an
    empty list means `publish = false` in the manifest, and a missing key means
    the same as `null`. Only the empty list is unpublishable.
    """
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    packages = json.loads(result.stdout)["packages"]
    publishable = [p for p in packages if p.get("publish") != []]
    return sorted(publishable, key=lambda p: str(p["name"]))


def main() -> int:
    packages = publishable_packages()
    if not packages:
        print("error: no publishable crates found", file=sys.stderr)
        return 1
    names_only = "--names" in sys.argv[1:]
    for package in packages:
        if names_only:
            print(package["name"])
            continue
        manifest = Path(str(package["manifest_path"]))
        try:
            relative = manifest.parent.relative_to(REPO_ROOT)
        except ValueError:
            relative = manifest.parent
        print(f"{package['name']}\t{package['version']}\t{relative.as_posix()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
