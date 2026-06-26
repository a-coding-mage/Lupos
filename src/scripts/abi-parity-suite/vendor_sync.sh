#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
LOCKFILE="$ROOT/src/docs/abi-parity-vendor-lock.tsv"

if [[ -d "$HOME/.local/bin" ]]; then
    case ":$PATH:" in
        *":$HOME/.local/bin:"*) ;;
        *) export PATH="$HOME/.local/bin:$PATH" ;;
    esac
fi

usage() {
    echo "usage: $0 --suite <suite>|all [--check|--fetch]" >&2
    exit 2
}

suite="all"
mode="check"
while (($#)); do
    case "$1" in
        --suite)
            shift
            (($#)) || usage
            suite="$1"
            ;;
        --check)
            mode="check"
            ;;
        --fetch)
            mode="fetch"
            ;;
        *)
            usage
            ;;
    esac
    shift
done

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: required command not found: $1" >&2
        exit 1
    }
}

field() {
    local key="$1"
    local column="$2"
    awk -F '\t' -v key="$key" -v column="$column" '$1 == key { print $column; exit }' "$LOCKFILE"
}

vendor_dir() {
    local key="$1"
    printf '%s/vendor/%s\n' "$ROOT" "$key"
}

download() {
    local url="$1"
    local out="$2"
    need_cmd curl
    curl -L --fail --silent --show-error "$url" -o "$out"
}

sha256_file() {
    need_cmd sha256sum
    sha256sum "$1" | awk '{print $1}'
}

extract_tarball() {
    local archive="$1"
    local dest="$2"
    local strip="${3:-1}"
    rm -rf "$dest"
    mkdir -p "$dest"
    if [[ "$strip" == "0" ]]; then
        tar -xf "$archive" -C "$dest"
    else
        tar -xf "$archive" -C "$dest" --strip-components="$strip"
    fi
}

fetch_github_archive() {
    local key="$1"
    local repo_url ref expected archive_url archive dest actual
    repo_url="$(field "$key" 3)"
    ref="$(field "$key" 4)"
    expected="$(field "$key" 5)"
    archive_url="${repo_url%.git}/archive/${ref}.tar.gz"
    archive="$(mktemp)"
    dest="$(vendor_dir "$key")"
    download "$archive_url" "$archive"
    actual="$(sha256_file "$archive")"
    if [[ "$actual" != "$expected" ]]; then
        echo "error: SHA256 mismatch for $key" >&2
        echo "expected: $expected" >&2
        echo "actual:   $actual" >&2
        exit 1
    fi
    extract_tarball "$archive" "$dest" 1
    rm -f "$archive"
}

fetch_xfstests_snapshot() {
    local key="xfstests"
    local version expected archive dest actual url
    version="$(field "$key" 2)"
    expected="$(field "$key" 5)"
    url="https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git/snapshot/xfstests-dev-${version}.tar.gz"
    archive="$(mktemp)"
    dest="$(vendor_dir "$key")"
    download "$url" "$archive"
    actual="$(sha256_file "$archive")"
    if [[ "$actual" != "$expected" ]]; then
        echo "error: SHA256 mismatch for $key" >&2
        echo "expected: $expected" >&2
        echo "actual:   $actual" >&2
        exit 1
    fi
    extract_tarball "$archive" "$dest" 1
    rm -f "$archive"
}

fetch_arch_rootfs() {
    local key="arch-rootfs"
    local url expected archive dest actual
    url="$(field "$key" 3)"
    expected="$(field "$key" 5)"
    archive="$(mktemp)"
    dest="$(vendor_dir "$key")"
    download "$url" "$archive"
    actual="$(sha256_file "$archive")"
    if [[ "$actual" != "$expected" ]]; then
        echo "error: SHA256 mismatch for $key" >&2
        echo "expected: $expected" >&2
        echo "actual:   $actual" >&2
        exit 1
    fi
    extract_tarball "$archive" "$dest" 1
    rm -f "$archive"
}

check_present() {
    local key="$1"
    local status
    status="$(field "$key" 6)"
    if [[ "$status" != "present" ]]; then
        return 0
    fi
    local dest
    dest="$(vendor_dir "$key")"
    [[ -e "$dest" ]] || {
        echo "error: vendor checkout missing for $key at $dest" >&2
        exit 1
    }
}

sync_one() {
    local key="$1"
    if [[ "$mode" == "check" ]]; then
        check_present "$key"
        return 0
    fi
    case "$key" in
        ltp|blktests|packetdrill|syzkaller)
            fetch_github_archive "$key"
            ;;
        xfstests)
            fetch_xfstests_snapshot
            ;;
        arch-rootfs)
            fetch_arch_rootfs
            ;;
        linux)
            check_present "$key"
            ;;
        *)
            echo "error: unsupported ABI parity vendor suite $key" >&2
            exit 1
            ;;
    esac
}

if [[ "$suite" == "all" ]]; then
    while IFS=$'\t' read -r key _; do
        [[ -n "$key" && "$key" != "suite" ]] || continue
        if [[ "$key" != "linux" ]]; then
            sync_one "$key"
        fi
    done < "$LOCKFILE"
else
    sync_one "$suite"
fi

echo "ABI parity vendor sync ($mode) completed for $suite"
