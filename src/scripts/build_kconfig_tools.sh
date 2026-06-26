#!/usr/bin/env bash
# Build vendor/linux/scripts/kconfig hostprogs (conf, mconf, nconf) outside
# of kbuild.
#
# Background:
#   The upstream scripts/kconfig/Makefile defines `conf`, `mconf`, and `nconf`
#   via kbuild's `hostprogs` machinery (vendor/linux/scripts/kconfig/Makefile
#   L174-L199).  Those targets only resolve when the build is invoked from
#   the Linux top-level Makefile, which sets up `HOSTCFLAGS_<obj>`,
#   `HOSTLDLIBS_<tool>`, and the `read-file` helper.  When we invoke
#   `make -C vendor/linux/scripts/kconfig <tool>` standalone, make falls
#   back to its implicit `%: %.c` rule and runs a bare `cc <tool>.c -o <tool>`
#   with no include path and no ncurses linkage.  `conf` happens to compile
#   that way, but `mconf` needs `<list.h>` from scripts/include and ncurses,
#   and `nconf` needs ncurses + menu + panel.
#
# This script reproduces the minimum subset of kbuild's hostprogs behaviour:
#   * The common-objs list and the per-tool object lists are copied verbatim
#     from vendor/linux/scripts/kconfig/Makefile L166-L192.
#   * Lexer/parser pre-generated sources (lexer.lex.c, parser.tab.c) are
#     committed in vendor, so no flex/bison is required.
#   * ncurses cflags/libs come from running the upstream mconf-cfg.sh /
#     nconf-cfg.sh helpers, which is what kbuild itself does.
#
# Usage:
#   scripts/build_kconfig_tools.sh <conf|mconf|nconf>
#
# Environment overrides (all optional):
#   HOSTCC           default: cc
#   HOSTPKG_CONFIG   default: pkg-config

set -eu

TOOL="${1:-}"
if [ -z "$TOOL" ]; then
	echo "usage: $0 <conf|mconf|nconf>" >&2
	exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KDIR="$REPO_ROOT/vendor/linux/scripts/kconfig"
INC_DIR="$REPO_ROOT/vendor/linux/scripts/include"

if [ ! -d "$KDIR" ]; then
	echo "*** kconfig sources missing: $KDIR" >&2
	echo "*** populate vendor/linux first (vendor/setup_linux.sh)" >&2
	exit 1
fi

export HOSTCC="${HOSTCC:-${CC:-cc}}"
export HOSTPKG_CONFIG="${HOSTPKG_CONFIG:-pkg-config}"

# Track temp files at script scope so a single EXIT trap can clean them up
# without tangling with set -u and bash RETURN-trap scope rules.
TMP_CFLAGS=""
TMP_LIBS=""
cleanup() {
	[ -z "$TMP_CFLAGS" ] || rm -f "$TMP_CFLAGS"
	[ -z "$TMP_LIBS" ] || rm -f "$TMP_LIBS"
}
trap cleanup EXIT

# Mirrors vendor/linux/scripts/kconfig/Makefile L166-L167:
#   common-objs := confdata.o expr.o lexer.lex.o menu.o parser.tab.o \
#                  preprocess.o symbol.o util.o
COMMON_SRCS=(confdata.c expr.c lexer.lex.c menu.c parser.tab.c preprocess.c symbol.c util.c)

# Mirrors vendor/linux/scripts/kconfig/Makefile L190-L192:
#   lxdialog := lxdialog/{checklist,inputbox,menubox,textbox,util,yesno}.o
LXDIALOG_SRCS=(
	lxdialog/checklist.c
	lxdialog/inputbox.c
	lxdialog/menubox.c
	lxdialog/textbox.c
	lxdialog/util.c
	lxdialog/yesno.c
)

# lexer.lex.c includes parser.tab.h, which lives next to it inside KDIR.
# scripts/include hosts list.h / hash.h / hashtable.h / xalloc.h consumed
# by mconf.c, confdata.c, preprocess.c.
KCONFIG_INC=("-I$KDIR" "-I$INC_DIR")

cd "$KDIR"

generate_parser_sources() {
	if [ ! -f parser.tab.c ] || [ ! -f parser.tab.h ]; then
		command -v bison >/dev/null 2>&1 || {
			echo "*** bison is required to generate scripts/kconfig/parser.tab.c" >&2
			exit 1
		}
		bison -t -d -o parser.tab.c parser.y
	fi

	if [ ! -f lexer.lex.c ]; then
		command -v flex >/dev/null 2>&1 || {
			echo "*** flex is required to generate scripts/kconfig/lexer.lex.c" >&2
			exit 1
		}
		flex -o lexer.lex.c lexer.l
	fi
}

# Reads one whitespace-separated word per token from $1 into the named array.
# Usage:  read_args_into <file> <array_name>
read_args_into() {
	local file="$1"
	local array_name="$2"
	# Clear the destination array; eval is the portable way to assign to a
	# name held in another variable across all bash versions in use.
	eval "$array_name=()"
	local line word
	while IFS= read -r line || [ -n "$line" ]; do
		for word in $line; do
			eval "$array_name+=(\"\$word\")"
		done
	done <"$file"
}

build_conf() {
	echo "*** building $KDIR/conf"
	generate_parser_sources
	"$HOSTCC" "${KCONFIG_INC[@]}" -o conf conf.c "${COMMON_SRCS[@]}"
}

build_mconf() {
	echo "*** building $KDIR/mconf"
	generate_parser_sources
	TMP_CFLAGS="$(mktemp)"
	TMP_LIBS="$(mktemp)"
	bash mconf-cfg.sh "$TMP_CFLAGS" "$TMP_LIBS"
	NCURSES_CFLAGS=()
	NCURSES_LIBS=()
	read_args_into "$TMP_CFLAGS" NCURSES_CFLAGS
	read_args_into "$TMP_LIBS" NCURSES_LIBS
	"$HOSTCC" "${KCONFIG_INC[@]}" ${NCURSES_CFLAGS[@]+"${NCURSES_CFLAGS[@]}"} \
		-o mconf \
		mconf.c mnconf-common.c \
		"${LXDIALOG_SRCS[@]}" \
		"${COMMON_SRCS[@]}" \
		${NCURSES_LIBS[@]+"${NCURSES_LIBS[@]}"}
}

build_nconf() {
	echo "*** building $KDIR/nconf"
	generate_parser_sources
	TMP_CFLAGS="$(mktemp)"
	TMP_LIBS="$(mktemp)"
	bash nconf-cfg.sh "$TMP_CFLAGS" "$TMP_LIBS"
	NCURSES_CFLAGS=()
	NCURSES_LIBS=()
	read_args_into "$TMP_CFLAGS" NCURSES_CFLAGS
	read_args_into "$TMP_LIBS" NCURSES_LIBS
	"$HOSTCC" "${KCONFIG_INC[@]}" ${NCURSES_CFLAGS[@]+"${NCURSES_CFLAGS[@]}"} \
		-o nconf \
		nconf.c nconf.gui.c mnconf-common.c \
		"${COMMON_SRCS[@]}" \
		${NCURSES_LIBS[@]+"${NCURSES_LIBS[@]}"}
}

case "$TOOL" in
	conf)  build_conf  ;;
	mconf) build_mconf ;;
	nconf) build_nconf ;;
	*)
		echo "*** unknown tool: $TOOL (expected conf|mconf|nconf)" >&2
		exit 2
		;;
esac
