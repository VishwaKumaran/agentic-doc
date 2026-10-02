#!/bin/sh
#
# Install the agentic-doc binary system-wide, so that every user on this
# machine finds it on their PATH.
#
# The binary is built from the checkout this script lives in: nothing is
# downloaded and no release is required.
#
# Usage: ./install.sh [--prefix DIR]
# Env:   AGENTIC_DOC_PREFIX   same as --prefix (the command line wins)

set -eu

PROG=agentic-doc
DEFAULT_PREFIX=/usr/local
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

Builds $PROG and installs <DIR>/bin/$PROG (default: $DEFAULT_PREFIX/bin/$PROG)
for every user on this machine.

Run from a checkout, the script builds that checkout. Piped to a shell (the
curl form above) there is no checkout: the source is downloaded first, so no
clone is needed.

Options:
  --prefix DIR   installation prefix (default: $DEFAULT_PREFIX)
  -h, --help     show this help

Environment:
  AGENTIC_DOC_PREFIX    same as --prefix; the command line wins
  AGENTIC_DOC_REPO      source repository (default: $DEFAULT_REPO)
  AGENTIC_DOC_REF       git ref to download (default: $DEFAULT_REF)
  AGENTIC_DOC_TARBALL   source tarball to use (path or URL) instead of GitHub
  AGENTIC_DOC_FETCH=0   never download; a local checkout is then required

Requires a Rust toolchain (https://rustup.rs), plus curl (or wget) and tar when
there is no checkout. Root privileges are required when <DIR>/bin cannot be
written by the current user.
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

prefix=${AGENTIC_DOC_PREFIX:-$DEFAULT_PREFIX}

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

# --- privileges --------------------------------------------------------------

bindir=$prefix/bin
binary=$bindir/$PROG

am_root=no
if [ "$(id -u)" -eq 0 ]; then
	am_root=yes
fi

# Building as the invoking user keeps target/ and the cargo home theirs and
# reuses their dependency cache; sudo tells us who invoked it.
invoker=${SUDO_USER:-}
work_as_user=no
if [ "$am_root" = yes ] && [ -n "$invoker" ] && [ "$invoker" != root ]; then
	work_as_user=yes
fi

# Walk up to the nearest directory that exists: creating the missing ones
# succeeds exactly when their closest existing ancestor is writable.
probe=$bindir
while [ ! -d "$probe" ]; do
	parent=$(dirname -- "$probe")
	if [ "$parent" = "$probe" ]; then
		break
	fi
	probe=$parent
done

writable=no
if [ "$am_root" = yes ] || [ -w "$probe" ]; then
	writable=yes
fi

if [ "$writable" = no ]; then
	if [ -f "$0" ]; then
		case $0 in
		*/*) rerun="sudo $0 --prefix $prefix" ;;
		*) rerun="sudo ./$0 --prefix $prefix" ;;
		esac
	else
		rerun="curl -fsSL https://raw.githubusercontent.com/$repo/$ref/install.sh | sudo sh -s -- --prefix $prefix"
	fi
	die "cannot write to $bindir (root privileges are required).
Re-run with:
  $rerun"
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
	# The tree was downloaded by root into a root-only directory; hand it to
	# the invoking user so the build below can run as them.
	if [ "$work_as_user" = yes ]; then
		chown -R "$invoker" "$cleanup_dir" 2>/dev/null || true
	fi
	printf 'Downloaded source: %s\n' "$checkout"
fi

cd "$checkout"

# --- locate cargo ------------------------------------------------------------

# sudo resets PATH (secure_path), which usually hides the user's cargo
# (typically ~/.cargo/bin); fall back to the invoking user's toolchain.
find_cargo() {
	if command -v cargo >/dev/null 2>&1; then
		command -v cargo
		return 0
	fi
	if [ -n "$invoker" ] && [ "$invoker" != root ]; then
		home=$(eval printf '%s' "~$invoker")
		if [ -x "$home/.cargo/bin/cargo" ]; then
			printf '%s\n' "$home/.cargo/bin/cargo"
			return 0
		fi
	fi
	return 1
}

cargo_bin=$(find_cargo) || die "cargo not found. Install a Rust toolchain (https://rustup.rs) and re-run."

# --- build -------------------------------------------------------------------

printf 'Building %s (release)...\n' "$PROG"

if [ "$work_as_user" = yes ]; then
	# Build as the invoking user so target/ and the cargo home stay theirs.
	if sudo -u "$invoker" -H "$cargo_bin" build --release; then
		:
	else
		printf 'Warning: could not build as %s; building as root.\n' "$invoker" >&2
		"$cargo_bin" build --release
	fi
else
	"$cargo_bin" build --release
fi

target_dir=${CARGO_TARGET_DIR:-target}
built=$target_dir/release/$PROG
[ -x "$built" ] || die "the build did not produce $built."

# --- install -----------------------------------------------------------------

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

# --- shadowing check ---------------------------------------------------------

# A copy earlier in PATH (e.g. ~/.cargo/bin on Linux) would hide this install.
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

case ":$PATH:" in
*":$bindir:"*) ;;
*) printf 'Warning: %s is not in PATH; add it to run %s as a command.\n' "$bindir" "$PROG" >&2 ;;
esac