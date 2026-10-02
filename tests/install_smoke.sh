#!/bin/sh
#
# Smoke test for install.sh. Runs the installer into a temporary prefix, with
# no root privileges and no network access, and checks the observable results.
#
# Usage: sh tests/install_smoke.sh

set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

# 1. A writable prefix needs no elevation: install end to end.
sh "$root/install.sh" --prefix "$tmp" || fail "install.sh --prefix exited non-zero"

[ -x "$tmp/bin/agentic-doc" ] || fail "binary was not installed in $tmp/bin"

version=$("$tmp/bin/agentic-doc" --version) || fail "installed binary does not run"
case $version in
agentic-doc*) ;;
*) fail "unexpected --version output: $version" ;;
esac

# 2. Re-running replaces the binary (update), not a failure.
sh "$root/install.sh" --prefix "$tmp" >/dev/null || fail "second install.sh run exited non-zero"

# 3. An unwritable prefix is refused with an explicit message, as a non-root user.
if [ "$(id -u)" -ne 0 ]; then
	out=$(sh "$root/install.sh" --prefix /usr/local 2>&1) && fail "install.sh should have refused to write to /usr/local"
	case $out in
	*sudo*) ;;
	*) fail "the refusal message does not mention sudo: $out" ;;
	esac
fi

# 4. --help works and lists --prefix.
sh "$root/install.sh" --help | grep -q -- '--prefix' || fail "--help does not document --prefix"

# 5. The curl form: pipe the script from a directory that holds no checkout.
#    AGENTIC_DOC_TARBALL keeps the test offline, standing in for GitHub.
tarball="$tmp/source.tar.gz"
tar -czf "$tarball" -C "$(dirname -- "$root")" --exclude=target --exclude=.git "$(basename -- "$root")" ||
	fail "could not build the source tarball for the piped test"

elsewhere="$tmp/elsewhere"
# Deliberately NOT created: the installer must create it via its closest
# writable ancestor instead of refusing.
piped="$tmp/nested/piped-prefix"
mkdir -p "$elsewhere"

if ! (cd "$elsewhere" && cat "$root/install.sh" | AGENTIC_DOC_TARBALL="$tarball" sh -s -- --prefix "$piped") >"$tmp/piped.log" 2>&1; then
	cat "$tmp/piped.log" >&2
	fail "the piped (curl-style) install failed"
fi

[ -x "$piped/bin/agentic-doc" ] || fail "the piped install did not produce the binary"
"$piped/bin/agentic-doc" --version >/dev/null || fail "the piped install binary does not run"

# 6. AGENTIC_DOC_FETCH=0 refuses to download an unrequested source.
if (cd "$elsewhere" && cat "$root/install.sh" | AGENTIC_DOC_FETCH=0 sh -s -- --prefix "$piped") >/dev/null 2>&1; then
	fail "AGENTIC_DOC_FETCH=0 should have refused to fetch a source"
fi

printf 'OK: install smoke test passed\n'