#!/usr/bin/env python3
"""Verifies a native package archive before it is published as a consumer release.

The target must be eligible in the release policy, and the archive must hold exactly the
files its `inventory.json` lists (written by `finalize-native-package.py` after signing),
with the sizes and SHA-256 digests listed there, all under one folder named like the archive
(`fframes-studio-<target>/` in `fframes-studio-<target>.zip`).
"""
import argparse
import hashlib
import json
import sys
import zipfile
from pathlib import Path


class Refused(Exception):
    pass


def verify(policy, target, archive):
    if target not in policy.get("eligible_targets", []):
        raise Refused(f"{target} is not eligible for consumer publication")
    root = f"{Path(archive).stem}/"
    with zipfile.ZipFile(archive) as package:
        entries = {}
        for item in package.infolist():
            if item.is_dir():
                continue
            if not item.filename.startswith(root) or ".." in item.filename.split("/"):
                raise Refused(f"{item.filename} is outside {root}")
            entries[item.filename.removeprefix(root)] = item
        if "inventory.json" not in entries:
            raise Refused(f"{root}inventory.json is missing")
        inventory = json.loads(package.read(entries.pop("inventory.json")))
        if inventory.get("target") != target:
            raise Refused(f"the inventory's target is {inventory.get('target')!r}, not {target}")
        listed = {item["path"]: item for item in inventory["files"]}
        for path in sorted(set(entries) - set(listed)):
            raise Refused(f"{path} is in the package but not in its inventory")
        for path in sorted(set(listed) - set(entries)):
            raise Refused(f"{path} is in the inventory but not in the package")
        for path, item in sorted(listed.items()):
            data = package.read(entries[path])
            if len(data) != item["size_bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
                raise Refused(f"{path} does not match its inventory entry")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("target")
    parser.add_argument("archive", type=Path)
    args = parser.parse_args(argv)
    try:
        verify(json.loads(args.policy.read_text()), args.target, args.archive)
    except Refused as refused:
        print(f"{args.archive}: {refused}", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
