#!/usr/bin/env python3
"""Check that the README badges point at things that exist.

A badge is a promise: "this crate is on crates.io", "these docs are
published". Until the first release none of the registry badges are true, and a
reader who follows one gets a 404 -- which is worse than no badge at all. This
script fetches every badge target in the repository's markdown and fails the
build when a *registry* entry is missing.

Two classes of target are treated differently on purpose:

- **Registry entries** (crates.io, PyPI, npm, docs.rs) must exist, or be listed
  in `.github/badge-allowlist.txt` with a reason. These are the claims a reader
  is most likely to act on.
- **Everything else** (CI status, coverage, the licence file) is only reported.
  A private repository answers 404 to an anonymous request even though the page
  is right there for a contributor, so failing on those would make the check
  unusable before the repository is public.

Usage:
    tools/check-badges.py [--offline] [FILE ...]

`--offline` prints what would be checked and exits 0, for a local run without
network access.
"""

from __future__ import annotations

import argparse
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ALLOWLIST = REPO_ROOT / ".github" / "badge-allowlist.txt"

# Host suffixes whose 404 means "this package is not published".
REGISTRY_HOSTS = (
    "crates.io",
    "pypi.org",
    "npmjs.com",
    "docs.rs",
)

# [![alt](image)](target) -- a badge that links somewhere.
BADGE_LINK = re.compile(
    r"\[!\[[^\]]*\]\((?P<image>[^)\s]+)\)\]\((?P<target>[^)\s]+)\)"
)
# ![alt](image) -- a badge or image with no link.
PLAIN_IMAGE = re.compile(r"!\[(?P<alt>[^\]]*)\]\((?P<image>[^)\s]+)\)")
MARKDOWN_FILES = ("README.md", "CHANGELOG.md")

USER_AGENT = "tpt-energy-badge-check (+https://github.com/tpt-solutions/tpt-energy)"
TIMEOUT_SECONDS = 20


def markdown_files() -> list[Path]:
    """Every markdown file that may carry a badge."""
    candidates = [REPO_ROOT / name for name in MARKDOWN_FILES]
    candidates += sorted(REPO_ROOT.glob("crates/**/README.md"))
    candidates += sorted(REPO_ROOT.glob("playground/*.md"))
    return [path for path in candidates if path.is_file()]


def badge_targets(path: Path) -> list[tuple[str, str]]:
    """`(url, alt)` for every badge image and badge link in `path`.

    A linked badge contributes both its image and its target: a shields.io
    image for an unpublished crate 404s, and so does the crates.io page it
    points at. Both are promises.
    """
    text = path.read_text(encoding="utf-8")
    found: list[tuple[str, str]] = []
    linked_images: set[str] = set()

    for match in BADGE_LINK.finditer(text):
        alt = match.group("target")
        found.append((match.group("image"), alt))
        found.append((match.group("target"), alt))
        linked_images.add(match.group("image"))

    for match in PLAIN_IMAGE.finditer(text):
        image = match.group("image")
        if image in linked_images:
            continue
        found.append((image, match.group("alt") or "image"))

    return found


def is_registry(url: str) -> bool:
    """True when a 404 for `url` means "not published yet"."""
    return any(host in url for host in REGISTRY_HOSTS)


def allowlisted(url: str) -> str | None:
    """The reason `url` is allowlisted, if it is."""
    if not ALLOWLIST.is_file():
        return None
    for line in ALLOWLIST.read_text(encoding="utf-8").splitlines():
        entry = line.strip()
        if not entry or entry.startswith("#"):
            continue
        parts = entry.split("#", 1)
        pattern = parts[0].strip()
        reason = parts[1].strip() if len(parts) > 1 else ""
        if pattern and pattern in url:
            return reason or "no reason given"
    return None


def status(url: str) -> int | None:
    """HTTP status for `url`, or `None` when it could not be fetched."""
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:
            return response.status
    except urllib.error.HTTPError as error:
        return error.code
    except (urllib.error.URLError, TimeoutError, OSError):
        return None



def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path, help="markdown files to check")
    parser.add_argument(
        "--offline",
        action="store_true",
        help="list the targets without fetching them",
    )
    args = parser.parse_args()

    files = args.files or markdown_files()
    if not files:
        print("error: no markdown files with badges found", file=sys.stderr)
        return 1

    failures: list[str] = []
    advisories: list[str] = []
    checked = 0

    for path in files:
        for target, alt in badge_targets(path):
            if not target.startswith("http"):
                continue
            relative = path.relative_to(REPO_ROOT).as_posix()
            checked += 1
            if args.offline:
                print(f"{relative}: {alt} -> {target}")
                continue
            code = status(target)
            if code is None:
                message = f"{relative}: could not reach {target} ({alt})"
                (failures if is_registry(target) else advisories).append(message)
                continue
            if 200 <= code < 400:
                continue
            reason = allowlisted(target)
            if reason is not None:
                print(f"note: {target} is {code} but allowlisted: {reason}")
                continue
            message = f"{relative}: {target} returned {code} ({alt})"
            (failures if is_registry(target) else advisories).append(message)

    for message in advisories:
        print(f"warning: {message}", file=sys.stderr)
    for message in failures:
        print(f"error: {message}", file=sys.stderr)

    if args.offline:
        print(f"{checked} badge target(s) would be checked")
        return 0
    if failures:
        print(
            "\nEither the package is not published yet, or the badge is wrong.\n"
            "Remove the badge until it is true, or add the URL to\n"
            f"{ALLOWLIST.relative_to(REPO_ROOT).as_posix()} with a reason.",
            file=sys.stderr,
        )
        return 1
    print(f"all {checked} badge target(s) resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
