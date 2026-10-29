#!/usr/bin/env python3
"""
Custom git merge driver: for binary files, always keep the version from the
most recently committed side (HEAD vs MERGE_HEAD). The file that was committed
later wins automatically — no conflict is raised.

Git calls this driver with:
  %O = ancestor/base (temp file)
  %A = ours / current branch (temp file — we write the result here)
  %B = theirs / incoming branch (temp file)
  %L = conflict-marker size (unused for binary)
  %P = pathname of the file being merged (relative to repo root)

Exit 0 = merge resolved; exit non-zero = unresolved conflict (git falls back).
"""

import subprocess
import shutil
import sys


def commit_timestamp(git_ref: str, path: str) -> int:
    """Unix timestamp of the last commit that touched `path` on `git_ref`.
    Returns 0 when the ref or path is not found (so the other side wins).
    """
    try:
        result = subprocess.run(
            ["git", "log", "-1", "--format=%ct", git_ref, "--", path],
            capture_output=True,
            text=True,
        )
        ts = result.stdout.strip()
        return int(ts) if ts else 0
    except Exception:
        return 0


def main() -> None:
    if len(sys.argv) < 6:
        print("merge-binary-newest: expected 5 arguments (O A B L P)", file=sys.stderr)
        sys.exit(1)

    _base  = sys.argv[1]   # %O ancestor
    ours   = sys.argv[2]   # %A current branch  ← we write the winner here
    theirs = sys.argv[3]   # %B incoming branch
    _size  = sys.argv[4]   # %L conflict-marker size
    path   = sys.argv[5]   # %P file path inside the repo

    ours_time   = commit_timestamp("HEAD",       path)
    theirs_time = commit_timestamp("MERGE_HEAD", path)

    if theirs_time > ours_time:
        # Incoming branch has the newer commit — use it.
        shutil.copy2(theirs, ours)
        print(f"merge-binary-newest: {path} — took MERGE_HEAD ({theirs_time} > {ours_time})",
              file=sys.stderr)
    else:
        # Current branch is newer (or equal) — keep ours as-is.
        print(f"merge-binary-newest: {path} — kept HEAD ({ours_time} >= {theirs_time})",
              file=sys.stderr)

    # Exit 0 = conflict resolved; git will not flag this file.
    sys.exit(0)


if __name__ == "__main__":
    main()

