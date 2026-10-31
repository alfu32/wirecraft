#!/usr/bin/env python3
"""Release only the selected existing tag; never invent or move tags."""

import os
import re
import subprocess
from common import ROOT, tag, version


def metadata(source_ref, toolchain):
    app_version = version(ROOT)
    if source_ref != "refs/tags/" + tag(app_version):
        raise ValueError(
            "Run this workflow at refs/tags/"
            + tag(app_version)
            + "; selected ref: "
            + source_ref
        )
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    tagged = subprocess.check_output(
        ["git", "rev-parse", source_ref + "^{commit}"], cwd=ROOT, text=True
    ).strip()
    if head != tagged:
        raise ValueError("Selected tag and checked-out source differ")
    # Pinned in the workflow, not by a rust-toolchain.toml that would also switch
    # local development tools (rust-analyzer) to that toolchain.
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", toolchain or ""):
        raise ValueError("Rust toolchain must be pinned to an exact X.Y.Z release")
    epoch = subprocess.check_output(
        ["git", "show", "-s", "--format=%ct", "HEAD"], cwd=ROOT, text=True
    ).strip()
    return dict(
        version=app_version,
        tag=tag(app_version),
        sha=head,
        toolchain=toolchain,
        epoch=epoch,
    )


if __name__ == "__main__":
    for k, v in metadata(
        os.environ.get("GITHUB_REF", ""), os.environ.get("RUST_TOOLCHAIN", "")
    ).items():
        print(k + "=" + v)
