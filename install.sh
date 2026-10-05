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
  AGENTIC_DOC_NO_COLOR=1 force plain output on a terminal (no colours, no bar)

No privileges are required. Requires a Rust toolchain (https://rustup.rs),
plus curl (or wget) and tar when there is no checkout.
EOF
}

die() {
	printf 'Error: %s\n' "$1" >&2
	exit 1
}

# --- presentation ------------------------------------------------------------
#
# The installer shows a small progress UI: a numbered list of steps and, for
# the build, a progress bar with the number of crates compiled and the elapsed
# time. Colours and animation are only enabled when stdout is a terminal, so a
# piped install (`curl ... | sh > log`) stays plain, greppable and
# deterministic -- nothing below depends on the display. Set
# AGENTIC_DOC_NO_COLOR=1 (or NO_COLOR=1) to force the plain form on a terminal.

plain=1
if [ -t 1 ] && [ -z "${AGENTIC_DOC_NO_COLOR:-}${NO_COLOR:-}" ]; then
	plain=0
fi

# Terminal width, clamped, so the bar never wraps. tput is optional.
columns=$(tput cols 2>/dev/null) || columns=80
case $columns in
'' | *[!0-9]*) columns=80 ;;
esac
[ "$columns" -ge 40 ] || columns=40

# Unicode glyphs only when the locale claims UTF-8 support.
utf8=0
case ${LC_ALL:-${LC_CTYPE:-${LANG:-}}} in
*UTF-8* | *utf8* | *UTF8* | *utf-8*) utf8=1 ;;
esac
if [ "$utf8" = 1 ]; then
	bar_full='█'
	bar_empty='░'
	mark_ok='✓'
	mark_tip='›'
else
	bar_full='#'
	bar_empty='-'
	mark_ok='ok'
	mark_tip='>'
fi

# Colours, unless the output is not a terminal or the user opted out.
c_reset= c_bold= c_dim= c_cyan= c_green= c_yellow= c_red=
if [ "$plain" = 0 ]; then
	esc=$(printf '\033')
	c_reset=$esc'[0m'
	c_bold=$esc'[1m'
	c_dim=$esc'[2m'
	c_cyan=$esc'[36m'
	c_green=$esc'[32m'
	c_yellow=$esc'[33m'
	c_red=$esc'[31m'
fi

step_total=6
step_current=0

# Banner shown once, before the first step.
ui_title() {
	if [ "$plain" = 0 ]; then
		printf '\n%s%s%s %sinstaller%s\n' \
			"$c_bold" "$PROG" "$c_reset" "$c_dim" "$c_reset"
	else
		printf '%s installer\n' "$PROG"
	fi
}

# Starts a new step, e.g. "[2/5] Locating the Rust toolchain".
step_begin() {
	step_current=$((step_current + 1))
	printf '\n%s[%d/%d]%s %s%s%s\n' \
		"$c_cyan" "$step_current" "$step_total" "$c_reset" "$c_bold" "$1" "$c_reset"
}

# A detail line under the current step.
step_note() {
	printf '      %s%s %s%s\n' "$c_dim" "$mark_tip" "$1" "$c_reset"
}

# An indented continuation line, e.g. a command to run by hand.
step_hint() {
	printf '        %s%s%s\n' "$c_dim" "$1" "$c_reset"
}

# Marks the current step as done.
step_ok() {
	printf '      %s%s%s %s\n' "$c_green" "$mark_ok" "$c_reset" "$1"
}

# Draws a bar of $3 columns, $1 of $2 units filled.
draw_bar() {
	filled=$1
	total=$2
	width=$3
	[ "$total" -gt 0 ] || total=1
	cols=$((filled * width / total))
	if [ "$cols" -gt "$width" ]; then
		cols=$width
	fi
	bar=
	i=0
	while [ "$i" -lt "$width" ]; do
		if [ "$i" -lt "$cols" ]; then
			bar=$bar$bar_full
		else
			bar=$bar$bar_empty
		fi
		i=$((i + 1))
	done
	printf '%s' "$bar"
}

# Draws a moving segment of $2 columns inside a bar of $3 columns.
sweep_bar() {
	pos=$1
	span=$2
	width=$3
	bar=
	i=0
	while [ "$i" -lt "$width" ]; do
		if [ "$i" -ge "$pos" ] && [ "$i" -lt $((pos + span)) ]; then
			bar=$bar$bar_full
		else
			bar=$bar$bar_empty
		fi
		i=$((i + 1))
	done
	printf '%s' "$bar"
}

# Runs a command under a progress bar and records the outcome in
# $build_crates / $build_secs for the caller to report. On a terminal the
# command's output is captured and replayed (tail) only if it fails; otherwise
# it streams straight through, keeping logs and pipes predictable.
build_with_progress() {
	build_crates=0
	build_secs=0
	start=$(date +%s)

	if [ "$plain" = 1 ]; then
		rc=0
		"$@" || rc=$?
		build_secs=$(( $(date +%s) - start ))
		return "$rc"
	fi

	# A cold build compiles about one unit per package in Cargo.lock, so that
	# count makes a usable total; without a lock file the bar sweeps instead.
	total=0
	if [ -f Cargo.lock ]; then
		total=$(grep -c '^\[\[package\]\]' Cargo.lock 2>/dev/null) || total=0
	fi

	log=$(mktemp 2>/dev/null) || {
		"$@"
		return $?
	}

	"$@" >"$log" 2>&1 &
	cpid=$!

	width=$((columns - 24))
	if [ "$width" -gt 32 ]; then
		width=32
	fi
	[ "$width" -ge 10 ] || width=10
	span=6
	pos=0
	pad=$(printf '%100s' '')

	while kill -0 "$cpid" 2>/dev/null; do
		crates=$(grep -c 'Compiling' "$log" 2>/dev/null) || crates=0
		secs=$(( $(date +%s) - start ))
		if [ "$crates" = 1 ]; then
			unit=crate
		else
			unit=crates
		fi
		if [ "$total" -gt 0 ]; then
			bar=$(draw_bar "$crates" "$total" "$width")
		else
			bar=$(sweep_bar "$pos" "$span" "$width")
		fi
		printf '\r      %s%s%s  %s%ss%s  %s%s %s%s' \
			"$c_cyan" "$bar" "$c_reset" \
			"$c_dim" "$secs" "$c_reset" \
			"$c_dim" "$crates" "$unit" "$c_reset"
		pos=$((pos + 1))
		if [ "$pos" -gt "$width" ]; then
			pos=0
		fi
		sleep 0.1 2>/dev/null || sleep 1
	done

	rc=0
	wait "$cpid" || rc=$?
	build_secs=$(( $(date +%s) - start ))
	build_crates=$(grep -c 'Compiling' "$log" 2>/dev/null) || build_crates=0

	printf '\r%s\r' "$pad"

	if [ "$rc" -ne 0 ]; then
		printf '      %sbuild failed after %ss%s\n' \
			"$c_red" "$build_secs" "$c_reset" >&2
		if [ -f "$log" ]; then
			tail -n 40 "$log" >&2 || true
		fi
		rm -f "$log"
		return "$rc"
	fi

	rm -f "$log"
	return 0
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

ui_title

step_begin "Locating source"

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
	step_note "downloaded $repo@$ref"
else
	step_note "checkout $checkout"
fi

cd "$checkout"

# --- locate cargo ------------------------------------------------------------

step_begin "Locating the Rust toolchain"
cargo_bin=$(command -v cargo) || die "cargo not found. Install a Rust toolchain (https://rustup.rs) and re-run."
step_ok "$("$cargo_bin" --version)"

step_begin "Checking C compiler"
if cc_bin=$(command -v cc 2>/dev/null) || cc_bin=$(command -v gcc 2>/dev/null) || cc_bin=$(command -v clang 2>/dev/null); then
	step_ok "found $cc_bin (required to compile tree-sitter grammars)"
else
	die "no C compiler found. Install cc, gcc, or clang, then re-run (tree-sitter grammars compile C)."
fi
# --- build -------------------------------------------------------------------

step_begin "Building $PROG (release)"
build_with_progress "$cargo_bin" build --release
if [ "${build_crates:-0}" = 1 ]; then
	step_ok "compiled in ${build_secs}s (${build_crates} crate)"
elif [ "${build_crates:-0}" -gt 1 ]; then
	step_ok "compiled in ${build_secs}s (${build_crates} crates)"
else
	step_ok "compiled in ${build_secs}s"
fi

target_dir=${CARGO_TARGET_DIR:-target}
built=$target_dir/release/$PROG
[ -x "$built" ] || die "the build did not produce $built."

# --- install -----------------------------------------------------------------

step_begin "Installing to $prefix"

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
	step_ok "Already up to date: $binary"
else
	install -m 0755 "$built" "$binary"
	step_ok "Installed: $binary"
	if [ "$before" != none ] && [ "$before" != unknown ]; then
		step_note "replaced $before"
	fi
fi

if after=$("$binary" --version 2>/dev/null); then
	:
else
	die "the installed binary at $binary failed to run."
fi

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
				step_ok "PATH already set in $rc"
				return 0
			fi
		done
	fi

	if printf '\n%s\nexport PATH="%s:$PATH"\n' "$marker" "$entry" >>"$rc" 2>/dev/null; then
		printf '      Added to %s:\n' "$rc"
		step_hint "export PATH=\"$entry:\$PATH\""
		printf '      Open a new terminal (or run that line) to use %s as a command.\n' "$PROG"
		step_ok "PATH updated"
	else
		printf '      %sWarning: could not update %s; add this line yourself:%s\n' \
			"$c_yellow" "$rc" "$c_reset" >&2
		step_hint "export PATH=\"$entry:\$PATH\"" >&2
	fi
}

step_begin "Configuring PATH"

case ":$PATH:" in
*":$bindir:"*)
	step_ok "$bindir is already in PATH"
	;;
*)
	if [ "${AGENTIC_DOC_NO_PATH:-0}" = 1 ]; then
		printf '      %sWarning: %s is not in PATH; add this line yourself:%s\n' \
			"$c_yellow" "$bindir" "$c_reset" >&2
		step_hint "export PATH=\"$bindir:\$PATH\"" >&2
		step_ok "PATH left unchanged (AGENTIC_DOC_NO_PATH=1)"
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
		printf '      %sWarning: an earlier copy in PATH may shadow this install: %s%s\n' \
			"$c_yellow" "$dir/$PROG" "$c_reset" >&2
		break
	fi
done

# --- done --------------------------------------------------------------------

printf '\n'
if [ "$plain" = 0 ]; then
	printf '%s%s%s %s%s%s is ready at %s\n' \
		"$c_green" "$mark_ok" "$c_reset" "$c_bold" "$after" "$c_reset" "$binary"
else
	printf '%s is ready at %s\n' "$after" "$binary"
fi
