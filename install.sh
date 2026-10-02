#!/bin/sh
#
# Install the agentic-doc binary for the current user, on their PATH.
#
# The binary is built from the checkout this script lives in: nothing is
# downloaded and no release is required.
#
# Usage: ./install.sh [--prefix DIR]
#        curl -fsSL https://raw.githubusercontent.com/VishwaKumaran/agentic-doc/main/install.sh | sh -s -- [--prefix DIR]
# Env:   AGENTIC_DOC_PREFIX    same as --prefix (the command line wins)
#        AGENTIC_DOC_NO_PATH=1 refuse to touch the shell profile

set -eu

PROG=agentic-doc
DEFAULT_REPO=VishwaKumaran/agentic-doc
DEFAULT_REF=main

repo=${AGENTIC_DOC_REPO:-$DEFAULT_REPO}
ref=${AGENTIC_DOC_REF:-$DEFAULT_REF}

cleanup_dir=
fetched_dir=

# The source is downloaded into a temporary directory when the script is piped
# to a shell instead of being run from a checkout; remove it on the way out.
trap 'if [ -n "$cleanup_dir" ]; then rm -rf "$cleanup_dir"; fi' 0

usage() {
	cat <<EOF
Usage: ./install.sh [--prefix DIR]
       curl -fsSL https://raw.githubusercontent.com/$DEFAULT_REPO/$DEFAULT_REF/install.sh | sh -s -- [--prefix DIR]

Builds $PROG and installs <DIR>/bin/$PROG for the current user.

Run from a checkout, the script builds that checkout. Piped to a shell (the
curl form above) there is no checkout: the source is downloaded first, so no
clone is needed.

Options:
  --prefix DIR   installation prefix (default: \$HOME/.local)
  -h, --help     show this help

Environment:
  AGENTIC_DOC_PREFIX     same as --prefix; the command line wins
  AGENTIC_DOC_REPO       source repository (default: $DEFAULT_REPO)
  AGENTIC_DOC_REF        git ref to download (default: $DEFAULT_REF)
  AGENTIC_DOC_TARBALL    source tarball to use (path or URL) instead of GitHub
  AGENTIC_DOC_FETCH=0    never download; a local checkout is then required
  AGENTIC_DOC_NO_PATH=1  do not add the installation directory to the PATH

No privileges are required. Requires a Rust toolchain (https://rustup.rs),
plus curl (or wget) and tar when there is no checkout.
EOF
}

die() {
	printf 'Error: %s\n' "$1" >&2
	exit 1
}

# --- obtaining the source ----------------------------------------------------

# A directory is a usable checkout only if it holds this very project: matching
# on the package name keeps a stray Cargo.toml from being built by mistake.
is_agentic_doc_checkout() {
	[ -f "$1/Cargo.toml" ] && [ -d "$1/src" ] &&
		grep -q '^name = "agentic-doc"' "$1/Cargo.toml" 2>/dev/null
}

download() {
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL -o "$2" "$1"
	elif command -v wget >/dev/null 2>&1; then
		wget -q -O "$2" "$1"
	else
		return 1
	fi
}

# Downloads (or copies) a source tarball and extracts it; sets fetched_dir.
fetch_source() {
	workdir=$(mktemp -d 2>/dev/null) || return 1
	cleanup_dir=$workdir
	tarball=$workdir/source.tar.gz

	if [ -n "${AGENTIC_DOC_TARBALL:-}" ]; then
		case $AGENTIC_DOC_TARBALL in
		*://*) download "$AGENTIC_DOC_TARBALL" "$tarball" || return 1 ;;
		*) cp -- "$AGENTIC_DOC_TARBALL" "$tarball" || return 1 ;;
		esac
	else
		download "https://github.com/$repo/archive/refs/heads/$ref.tar.gz" "$tarball" || return 1
	fi

	tar -xzf "$tarball" -C "$workdir" || return 1

	manifest=$(find "$workdir" -maxdepth 2 -type f -name Cargo.toml 2>/dev/null | head -n 1)
	[ -n "$manifest" ] || return 1
	fetched_dir=$(dirname -- "$manifest")
}
# --- arguments ---------------------------------------------------------------

[ -n "${HOME:-}" ] || die "HOME is not set; pass --prefix explicitly."

# ~/.local is the XDG user directory whose bin/ subdirectory holds per-user
# executables, so no privileges are ever needed.
prefix=${AGENTIC_DOC_PREFIX:-$HOME/.local}

while [ $# -gt 0 ]; do
	case $1 in
	--prefix)
		[ $# -ge 2 ] || die "--prefix requires a directory."
		prefix=$2
		shift 2
		;;
	--prefix=*)
		prefix=${1#--prefix=}
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		die "unknown argument: $1 (try --help)"
		;;
	esac
done

[ -n "$prefix" ] || die "--prefix must not be empty."

# Make the prefix absolute before changing directory.
case $prefix in
/*) ;;
*) prefix=$PWD/$prefix ;;
esac

# --- a single user's install -------------------------------------------------

# The destination belongs to one account, so root is never needed and running
# as root would install into root's home instead of the user's.
if [ "$(id -u)" -eq 0 ]; then
	die "this installs $PROG for a single user; do not run it as root (it would install into root's home).
Re-run without sudo."
fi

# --- locate the checkout -----------------------------------------------------

# Running as ./install.sh, $0 is the script; piped to a shell, $0 is the shell
# and the current directory is the only candidate.
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || script_dir=$PWD

checkout=
if is_agentic_doc_checkout "$script_dir"; then
	checkout=$script_dir
elif is_agentic_doc_checkout "$PWD"; then
	checkout=$PWD
fi

if [ -z "$checkout" ]; then
	if [ "${AGENTIC_DOC_FETCH:-1}" = 0 ]; then
		die "not an agentic-doc checkout (Cargo.toml + src) and AGENTIC_DOC_FETCH=0."
	fi
	if ! fetch_source; then
		die "could not obtain the agentic-doc source: curl (or wget), tar and network are required outside a checkout."
	fi
	checkout=$fetched_dir
	printf 'Downloaded source: %s\n' "$checkout"
fi

cd "$checkout"

# --- locate cargo ------------------------------------------------------------

cargo_bin=$(command -v cargo) || die "cargo not found. Install a Rust toolchain (https://rustup.rs) and re-run."

# --- build -------------------------------------------------------------------

printf 'Building %s (release)...\n' "$PROG"
"$cargo_bin" build --release

target_dir=${CARGO_TARGET_DIR:-target}
built=$target_dir/release/$PROG
[ -x "$built" ] || die "the build did not produce $built."

# --- install -----------------------------------------------------------------

bindir=$prefix/bin
binary=$bindir/$PROG

mkdir -p "$bindir"

before=none
if [ -x "$binary" ]; then
	if before=$("$binary" --version 2>/dev/null); then
		:
	else
		before=unknown
	fi
fi

if [ -x "$binary" ] && cmp -s "$built" "$binary"; then
	printf 'Already up to date: %s\n' "$binary"
else
	install -m 0755 "$built" "$binary"
	printf 'Installed: %s\n' "$binary"
fi

if after=$("$binary" --version 2>/dev/null); then
	:
else
	die "the installed binary at $binary failed to run."
fi

printf 'Version:   %s (before: %s)\n' "$after" "$before"

# --- PATH --------------------------------------------------------------------

# Prepending puts this copy ahead of any other one, such as a system-wide
# /usr/local/bin/agentic-doc left over from an earlier install.
profile_file() {
	case $(basename -- "${SHELL:-/bin/sh}") in
	zsh) printf '%s\n' "$HOME/.zshrc" ;;
	bash) printf '%s\n' "$HOME/.bashrc" ;;
	*) printf '%s\n' "$HOME/.profile" ;;
	esac
}

add_to_path() {
	rc=$1

	# $HOME inside the prefix keeps the line valid if the home moves later.
	entry=$bindir
	case $bindir in
	"$HOME"/*) entry="\$HOME/${bindir#"$HOME"/}" ;;
	esac

	marker='# added by agentic-doc install.sh'

	if [ -f "$rc" ]; then
		for needle in "$bindir" "$entry" "$marker"; do
			if grep -Fq "$needle" "$rc"; then
				printf 'PATH already set in %s\n' "$rc"
				return 0
			fi
		done
	fi

	if printf '\n%s\nexport PATH="%s:$PATH"\n' "$marker" "$entry" >>"$rc" 2>/dev/null; then
		printf 'Added to PATH in %s:\n  export PATH="%s:$PATH"\n' "$rc" "$entry"
		printf 'Open a new terminal (or run that line) to use %s as a command.\n' "$PROG"
	else
		printf 'Warning: could not update %s; add this line yourself:\n  export PATH="%s:$PATH"\n' "$rc" "$entry" >&2
	fi
}

case ":$PATH:" in
*":$bindir:"*)
	printf '%s is already in PATH\n' "$bindir"
	;;
*)
	if [ "${AGENTIC_DOC_NO_PATH:-0}" = 1 ]; then
		printf 'Warning: %s is not in PATH; add this line yourself:\n  export PATH="%s:$PATH"\n' "$bindir" "$bindir" >&2
	else
		add_to_path "$(profile_file)"
	fi
	;;
esac

# --- shadowing check ---------------------------------------------------------

# Anything earlier in PATH than this install would hide it.
for dir in $(printf '%s' "$PATH" | tr ':' ' '); do
	[ -n "$dir" ] || continue
	if [ "$dir" = "$bindir" ]; then
		break
	fi
	if [ -x "$dir/$PROG" ]; then
		printf 'Warning: an earlier copy in PATH may shadow this install: %s\n' "$dir/$PROG" >&2
		break
	fi
done