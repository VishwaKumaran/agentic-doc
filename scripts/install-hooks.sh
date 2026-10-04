#!/bin/sh
#
# Point git at the version-controlled hooks in scripts/git-hooks, so the
# pre-commit version bump works in this checkout.
#
# Usage: sh scripts/install-hooks.sh

set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

git -C "$root" config core.hooksPath scripts/git-hooks
chmod +x "$root"/scripts/git-hooks/* 2>/dev/null || true

printf 'install-hooks: core.hooksPath = scripts/git-hooks\n'
