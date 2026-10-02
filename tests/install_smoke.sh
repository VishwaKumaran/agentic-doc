#!/bin/sh
#
# Smoke test for install.sh. Runs the installer into temporary prefixes and a
# temporary HOME, with no privileges and no network access, and checks the
# observable results.
#
# Usage: sh tests/install_smoke.sh

set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# The cases below override HOME to keep the real shell profile untouched. That
# would also hide rustup's toolchain, because RUSTUP_HOME defaults to
# $HOME/.rustup, so pin both toolchain homes to their real values here.
export CARGO_HOME=${CARGO_HOME:-${HOME:-}/.cargo}
export RUSTUP_HOME=${RUSTUP_HOME:-${HOME:-}/.rustup}

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

# Each case gets its own HOME so the shell profile is never a real one.
fresh_home() {
	home=$tmp/home-$1
	mkdir -p "$home"
	printf '%s\n' "$home"
}

# 1. Install from the checkout into a temporary prefix.
home1=$(fresh_home install)
AGENTIC_DOC_NO_PATH=1 HOME="$home1" sh "$root/install.sh" --prefix "$tmp/prefix" ||
	fail "install.sh exited non-zero"

[ -x "$tmp/prefix/bin/agentic-doc" ] || fail "binary was not installed in $tmp/prefix/bin"
"$tmp/prefix/bin/agentic-doc" --version >/dev/null || fail "the installed binary does not run"

# 2. Re-running replaces the binary (update), not a failure.
AGENTIC_DOC_NO_PATH=1 HOME="$home1" sh "$root/install.sh" --prefix "$tmp/prefix" >"$tmp/second.log" 2>&1 ||
	fail "the second install.sh run exited non-zero"
grep -q "Already up to date" "$tmp/second.log" || fail "the second run did not report an up-to-date install"

# 3. The curl form: pipe the script from a directory that holds no checkout.
#    AGENTIC_DOC_TARBALL keeps the test offline, standing in for GitHub.
tarball="$tmp/source.tar.gz"
tar -czf "$tarball" -C "$(dirname -- "$root")" --exclude=target --exclude=.git "$(basename -- "$root")" ||
	fail "could not build the source tarball for the piped test"

elsewhere=$tmp/elsewhere
mkdir -p "$elsewhere"
home2=$(fresh_home path)

# The prefix is deliberately not created yet: the installer must create it.
piped_prefix=$tmp/nested/piped-prefix
if ! (
	cd "$elsewhere" &&
		HOME="$home2" SHELL=/bin/zsh AGENTIC_DOC_TARBALL="$tarball" \
			sh -c "cat '$root/install.sh' | sh -s -- --prefix '$piped_prefix'"
) >"$tmp/piped.log" 2>&1; then
	cat "$tmp/piped.log" >&2
	fail "the piped (curl-style) install failed"
fi

[ -x "$piped_prefix/bin/agentic-doc" ] || fail "the piped install did not produce the binary"
"$piped_prefix/bin/agentic-doc" --version >/dev/null || fail "the piped install binary does not run"

# 4. The pipe added the install directory to the shell profile, prepended.
rc=$home2/.zshrc
[ -f "$rc" ] || fail "the installer did not create $rc"
grep -Fq "$piped_prefix/bin" "$rc" || fail "the installer did not add the install directory to $rc"
marker_lines=$(grep -c 'added by agentic-doc install.sh' "$rc" || true)
[ "$marker_lines" = 1 ] || fail "expected exactly one marker line in $rc, found $marker_lines"

# 5. A second run is idempotent: no duplicate PATH line.
(
	cd "$elsewhere" &&
		HOME="$home2" SHELL=/bin/zsh AGENTIC_DOC_TARBALL="$tarball" \
			sh -c "cat '$root/install.sh' | sh -s -- --prefix '$piped_prefix'"
) >/dev/null 2>&1 || fail "the second piped run exited non-zero"
marker_lines=$(grep -c 'added by agentic-doc install.sh' "$rc" || true)
[ "$marker_lines" = 1 ] || fail "the second run duplicated the PATH line in $rc"

# 6. AGENTIC_DOC_NO_PATH=1 leaves the profile alone.
home3=$(fresh_home nopath)
AGENTIC_DOC_NO_PATH=1 HOME="$home3" SHELL=/bin/zsh sh "$root/install.sh" --prefix "$tmp/nopath" >"$tmp/nopath.log" 2>&1 ||
	fail "the AGENTIC_DOC_NO_PATH run exited non-zero"
if [ -f "$home3/.zshrc" ]; then
	fail "AGENTIC_DOC_NO_PATH=1 still created a profile file"
fi
grep -q "not in PATH" "$tmp/nopath.log" || fail "AGENTIC_DOC_NO_PATH=1 did not warn about PATH"

# 7. AGENTIC_DOC_FETCH=0 refuses to download an unrequested source.
if (cd "$elsewhere" && AGENTIC_DOC_FETCH=0 sh -c "cat '$root/install.sh' | sh -s -- --prefix '$tmp/nowhere'") >/dev/null 2>&1; then
	fail "AGENTIC_DOC_FETCH=0 should have refused to fetch a source"
fi

# 8. Running as root is refused: a per-user install must not run under sudo.
fakebin=$tmp/fakebin
mkdir -p "$fakebin"
printf '#!/bin/sh\necho 0\n' >"$fakebin/id"
chmod +x "$fakebin/id"
if PATH="$fakebin:$PATH" HOME="$home1" sh "$root/install.sh" --prefix "$tmp/root-prefix" >"$tmp/root.log" 2>&1; then
	fail "install.sh should have refused to run as root"
fi
grep -q "do not run it as root" "$tmp/root.log" || fail "the root refusal message is missing"

# 9. --help works and lists --prefix.
sh "$root/install.sh" --help | grep -q -- '--prefix' || fail "--help does not document --prefix"

printf 'OK: install smoke test passed\n'