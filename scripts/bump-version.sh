#!/bin/sh
#
# Increment the CLI version in Cargo.toml ([package].version) and keep
# Cargo.lock in sync, so every commit ships a new version. The touched files
# are staged so they are picked up by the commit being prepared.
#
# Usage: sh scripts/bump-version.sh [--major|--minor|--patch]
#        (default: --patch)
# Env:   AGENTIC_DOC_SKIP_BUMP=1   do nothing (escape hatch)

set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cargo_toml=$root/Cargo.toml
cargo_lock=$root/Cargo.lock

part=patch
for arg in "$@"; do
	case $arg in
		--major) part=major ;;
		--minor) part=minor ;;
		--patch) part=patch ;;
		*) printf 'bump-version: unknown argument: %s\n' "$arg" >&2; exit 2 ;;
	esac
done

# Escape hatch for tooling commits that must not touch the version.
if [ "${AGENTIC_DOC_SKIP_BUMP:-0}" = "1" ]; then
	exit 0
fi

[ -f "$cargo_toml" ] || { printf 'bump-version: %s not found\n' "$cargo_toml" >&2; exit 1; }

# Read the version declared in the [package] section of Cargo.toml.
current=$(awk '
	/^\[/ { in_pkg = ($0 == "[package]") }
	in_pkg && $0 ~ /^version[[:space:]]*=/ { line = $0 }
	END { if (line != "") print line }
' "$cargo_toml" | sed -e 's/^[^"]*"//' -e 's/".*$//')

case $current in
	[0-9]*.[0-9]*.[0-9]*) ;;
	*) printf 'bump-version: cannot parse version from %s\n' "$cargo_toml" >&2; exit 1 ;;
esac

major=$(printf '%s' "$current" | cut -d. -f1)
minor=$(printf '%s' "$current" | cut -d. -f2)
patch=$(printf '%s' "$current" | cut -d. -f3)

case $part in
	major) major=$((major + 1)); minor=0; patch=0 ;;
	minor) minor=$((minor + 1)); patch=0 ;;
	patch) patch=$((patch + 1)) ;;
esac

next=$major.$minor.$patch

# Rewrite only the version line inside [package] in Cargo.toml.
awk -v old="$current" -v new="$next" '
	/^\[/ { in_pkg = ($0 == "[package]") }
	in_pkg && !done && $0 ~ /^version[[:space:]]*=/ {
		sub("\"" old "\"", "\"" new "\"")
		done = 1
	}
	{ print }
' "$cargo_toml" > "$cargo_toml.tmp"
mv "$cargo_toml.tmp" "$cargo_toml"

# Keep Cargo.lock in sync when the package entry is present.
if [ -f "$cargo_lock" ]; then
	awk -v new="$next" '
		/^name = "agentic-doc"$/ { seen = 1 }
		seen && /^version = / { sub(/"[^"]*"/, "\"" new "\""); seen = 0 }
		{ print }
	' "$cargo_lock" > "$cargo_lock.tmp"
	mv "$cargo_lock.tmp" "$cargo_lock"
fi

# Stage the touched files so they belong to the commit being prepared.
if git -C "$root" rev-parse --git-dir >/dev/null 2>&1; then
	git -C "$root" add -- "$cargo_toml"
	if [ -f "$cargo_lock" ]; then
		git -C "$root" add -- "$cargo_lock"
	fi
fi

printf 'bump-version: %s -> %s\n' "$current" "$next"
