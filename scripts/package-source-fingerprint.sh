#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
python3 - "$ROOT" <<'PY'
import hashlib
import os
import sys
from pathlib import Path

root = Path(sys.argv[1])
files = []
for directory in (root / "src", root / "src-tauri"):
    for path in directory.rglob("*"):
        if not path.is_file() or "target" in path.relative_to(root).parts:
            continue
        files.append(path)
for name in ("index.html", "package.json", "package-lock.json", "tsconfig.json", "vite.config.ts"):
    path = root / name
    if path.is_file():
        files.append(path)
digest = hashlib.sha256()
for path in sorted(set(files), key=lambda item: item.relative_to(root).as_posix()):
    relative = path.relative_to(root).as_posix().encode()
    digest.update(relative + b"\0")
    digest.update(oct(os.stat(path).st_mode & 0o777).encode() + b"\0")
    digest.update(path.read_bytes())
    digest.update(b"\0")
print(digest.hexdigest())
PY
