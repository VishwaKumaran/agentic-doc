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

usage() {
	cat <<EOF
Usage: ./install.sh [--prefix DIR]

Builds $PROG from this checkout and installs <DIR>/bin/$PROG
(default: $DEFAULT_PREFIX/bin/$PROG) for every user on this machine.

Options:
  --prefix DIR   installation prefix (default: $DEFAULT_PREFIX)
  -h, --help     show this help

Environment:
  AGENTIC_DOC_PREFIX   same as --prefix; the command line wins

Requires a Rust toolchain (https://rustup.rs). Root privileges are required
when <DIR>/bin cannot be written by the current user.
EOF
}

die() {
	printf 'Error: %s\n' "$1" >&2
	exit 1
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

# --- locate the checkout -----------------------------------------------------

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir"

[ -f Cargo.toml ] || die "Cargo.toml not found in $script_dir; run this script from the repository."
[ -d src ] || die "src/ not found in $script_dir; run this script from the repository."

# --- locate cargo ------------------------------------------------------------

# sudo resets PATH (secure_path), which usually hides the user's cargo
# (typically ~/.cargo/bin); fall back to the invoking user's toolchain.
find_cargo() {
	if command -v cargo >/dev/null 2>&1; then
		command -v cargo
		return 0
	fi
	invoker=${SUDO_USER:-}
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

# --- privileges --------------------------------------------------------------

bindir=$prefix/bin
binary=$bindir/$PROG

am_root=no
if [ "$(id -u)" -eq 0 ]; then
	am_root=yes
fi

writable=no
if [ "$am_root" = yes ]; then
	writable=yes
elif [ -d "$bindir" ]; then
	if [ -w "$bindir" ]; then writable=yes; fi
elif [ -w "$prefix" ]; then
	writable=yes
fi

if [ "$writable" = no ]; then
	die "cannot write to $bindir (root privileges are required).
Re-run with:
  sudo $0 --prefix $prefix"
fi

# --- build -------------------------------------------------------------------

printf 'Building %s (release)...\n' "$PROG"

if [ "$am_root" = yes ] && [ -n "${SUDO_USER:-}" ] && [ "${SUDO_USER:-}" != root ]; then
	# Build as the invoking user so target/ and the cargo home stay theirs.
	if sudo -u "$SUDO_USER" -H "$cargo_bin" build --release; then
		:
	else
		printf 'Warning: could not build as %s; building as root.\n' "$SUDO_USER" >&2
		"$cargo_bin" build --release
	fi
else
	"$cargo_bin" build --release
fi

built=target/release/$PROG
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