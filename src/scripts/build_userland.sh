#!/usr/bin/env bash
# Materialize an Arch Linux base rootfs for the Lupos ISO/initramfs.
#
# This is intentionally unprivileged: no sudo, no chroot, no mounts, and no
# writes outside target/userland. The Arch bootstrap tarball is the same base
# input used for a fresh-host Arch bootstrap; Lupos only stages it and applies
# the guest overlay needed by our boot path.
#
# Usage:
#   bash src/scripts/build_userland.sh           # skip if stage exists
#   LUPOS_ARCH_REFRESH=1 bash ...                # force restage/re-extract
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

LUPOS_DISTRO="${LUPOS_DISTRO:-arch}"
BOOTSTRAP_NAME="${LUPOS_ARCH_BOOTSTRAP_NAME:-archlinux-bootstrap-2026.06.01-x86_64}"
BOOTSTRAP_SHA256="${LUPOS_ARCH_BOOTSTRAP_SHA256:-e68ba918c9f7deede8eccd2cd8ce259df104d84b0791cff3a2bc7579ced34849}"
BOOTSTRAP_URL="${LUPOS_ARCH_BOOTSTRAP_URL:-https://archive.archlinux.org/iso/2026.06.01/${BOOTSTRAP_NAME}.tar.zst}"
ARCH_REPO_SNAPSHOT="2026/06/01"
ARCH_REPO_BASE_URL="${LUPOS_ARCH_REPO_BASE_URL:-https://archive.archlinux.org/repos/$ARCH_REPO_SNAPSHOT}"
ARCH_OFFLINE_REPO_REL="var/lib/lupos/pacman-repo"
ARCH_PACMAN_SERVER='Server = file:///var/lib/lupos/pacman-repo/$repo/os/$arch'
ARCH_PACMAN_XFER_HELPER="usr/lib/lupos/pacman-xfer"
ARCH_PACMAN_XFER_COMMAND="XferCommand = /usr/lib/lupos/pacman-xfer %u %o"
ARCH_OFFLINE_REPO_ARTIFACTS=(
    "core/os/x86_64/core.db:a7e2c5c084e9bf7db0a5c9231942a4bd12cf170b85ae8d048e10d75acbe74e4d"
    "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst:95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    "extra/os/x86_64/extra.db:2b5a0cb4a6e5503a060c6011fb6cacb58c3b990a44829c3b70646641117b94c4"
    "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst:f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst:96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
)
ARCH_OFFLINE_REPO_PACKAGES=(
    "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst:95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst:f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst:96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
)
ARCH_OFFLINE_REPO_PACKAGE_ALIASES=(
    "p/g:/var/lib/lupos/pacman-repo/core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst"
    "p/v:/var/lib/lupos/pacman-repo/extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst"
    "p/r:/var/lib/lupos/pacman-repo/extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst"
)

TARGET="$ROOT/target/userland"
CACHE="$TARGET/cache"
BOOTSTRAP_ARCHIVE="$CACHE/${BOOTSTRAP_NAME}.tar.zst"
LUPOS_ARCH_ROOTFS="${LUPOS_ARCH_ROOTFS:-$ROOT/target/userland/arch-rootfs}"
LUPOS_ARCH_BOOTSTRAP_ROOTFS="${LUPOS_ARCH_BOOTSTRAP_ROOTFS:-}"
ARCH_ROOTFS="${LUPOS_ARCH_BOOTSTRAP_ROOTFS:-$LUPOS_ARCH_ROOTFS}"
STAGE="${STAGE:-$TARGET/stage}"
STAGE_STAMP="$STAGE/.lupos-userland-ok"

die() { echo "error: $*" >&2; exit 1; }
log() { echo "==> $*"; }

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "$1 required"
}

ensure_unprivileged() {
    [ "$(id -u)" != "0" ] || die "refusing to run as root; run this as your normal user"
    [ -z "${SUDO_UID:-}" ] || die "refusing to run under sudo; run this as your normal user"
}

safe_clean_dir() {
    local dir="$1"
    case "$dir" in
        "$TARGET"/*|"$TARGET") ;;
        *) die "refusing to remove path outside target/userland: $dir" ;;
    esac
    if [ -e "$dir" ] && [ ! -O "$dir" ]; then
        die "$dir is not owned by the current user; not removing it without sudo"
    fi
    [ ! -e "$dir" ] || chmod -R u+rwX "$dir"
    rm -rf "$dir"
}

rootfs_ready() {
    [ -e "$ARCH_ROOTFS/usr/lib/systemd/systemd" ] \
        && [ -e "$ARCH_ROOTFS/usr/bin/bash" ] \
        && [ -e "$ARCH_ROOTFS/usr/bin/pacman" ] \
        && [ -e "$ARCH_ROOTFS/etc/pacman.conf" ] \
        && [ -e "$ARCH_ROOTFS/var/lib/pacman/local/ALPM_DB_VERSION" ]
}

make_rootfs_user_writable() {
    chmod -R u+rwX "$ARCH_ROOTFS"
}

pam_systemd_ready() {
    local r="$1"
    grep -q '^session optional pam_systemd\.so$' "$r/etc/pam.d/system-login" \
        && grep -q 'pam_systemd\.so' "$r/usr/lib/pam.d/systemd-user"
}

pacman_mirrorlist_ready() {
    local r="$1"
    grep -Fxq "$ARCH_PACMAN_SERVER" "$r/etc/pacman.d/mirrorlist"
}

pacman_repo_siglevel_ready() {
    local conf="$1"
    local repo="$2"
    awk -v repo="$repo" '
        $0 == "[" repo "]" { in_repo = 1; next }
        /^\[/ { in_repo = 0 }
        in_repo && $0 == "SigLevel = Optional TrustAll" { found = 1 }
        END { exit found ? 0 : 1 }
    ' "$conf"
}

pacman_repo_server_ready() {
    local conf="$1"
    local repo="$2"
    awk -v repo="$repo" -v server="$ARCH_PACMAN_SERVER" '
        $0 == "[" repo "]" { in_repo = 1; next }
        /^\[/ { in_repo = 0 }
        in_repo && $0 == server { found = 1 }
        END { exit found ? 0 : 1 }
    ' "$conf"
}

pacman_config_ready() {
    local r="$1"
    grep -Eq '^[[:space:]]*DisableSandbox[[:space:]]*$' "$r/etc/pacman.conf" \
        && ! grep -Eq '^[[:space:]]*DownloadUser[[:space:]]*=' "$r/etc/pacman.conf" \
        && grep -Fxq "$ARCH_PACMAN_XFER_COMMAND" "$r/etc/pacman.conf" \
        && [ -x "$r/$ARCH_PACMAN_XFER_HELPER" ] \
        && pacman_repo_siglevel_ready "$r/etc/pacman.conf" core \
        && pacman_repo_siglevel_ready "$r/etc/pacman.conf" extra \
        && pacman_repo_server_ready "$r/etc/pacman.conf" core \
        && pacman_repo_server_ready "$r/etc/pacman.conf" extra
}

pacman_offline_repo_ready() {
    local r="$1"
    local entry rel sha path actual target repo_db sync_db
    for entry in "${ARCH_OFFLINE_REPO_ARTIFACTS[@]}"; do
        rel="${entry%%:*}"
        sha="${entry##*:}"
        path="$r/$ARCH_OFFLINE_REPO_REL/$rel"
        [ -s "$path" ] || return 1
        actual="$(sha256_of "$path")"
        [ "$actual" = "$sha" ] || return 1
    done
    for entry in "${ARCH_OFFLINE_REPO_PACKAGES[@]}"; do
        rel="${entry%%:*}"
        path="$r/var/cache/pacman/pkg/$(basename "$rel")"
        [ ! -e "$path" ] || return 1
    done
    for entry in "${ARCH_OFFLINE_REPO_PACKAGE_ALIASES[@]}"; do
        rel="${entry%%:*}"
        target="${entry#*:}"
        path="$r/$rel"
        [ "$(readlink "$path" 2>/dev/null)" = "$target" ] || return 1
    done
    for repo in core extra; do
        repo_db="$r/$ARCH_OFFLINE_REPO_REL/$repo/os/x86_64/$repo.db"
        sync_db="$r/var/lib/pacman/sync/$repo.db"
        [ -s "$sync_db" ] || return 1
        cmp -s "$repo_db" "$sync_db" || return 1
    done
}

stage_ready() {
    [ -e "$STAGE_STAMP" ] \
        && [ -e "$STAGE/.lupos-profile" ] \
        && [ -e "$STAGE/usr/lib/systemd/systemd" ] \
        && [ -e "$STAGE/usr/bin/bash" ] \
        && [ -e "$STAGE/usr/bin/pacman" ] \
        && [ -e "$STAGE/etc/systemd/network/10-lupos-qemu.network" ] \
        && pacman_mirrorlist_ready "$STAGE" \
        && pacman_config_ready "$STAGE" \
        && pacman_offline_repo_ready "$STAGE" \
        && pam_systemd_ready "$STAGE"
}

download_bootstrap() {
    mkdir -p "$CACHE"
    if [ -f "$BOOTSTRAP_ARCHIVE" ]; then
        local actual
        actual="$(sha256_of "$BOOTSTRAP_ARCHIVE")"
        [ "$actual" = "$BOOTSTRAP_SHA256" ] || die \
            "cached bootstrap checksum mismatch: expected $BOOTSTRAP_SHA256, got $actual; remove $BOOTSTRAP_ARCHIVE to download again"
        log "Using cached bootstrap archive $BOOTSTRAP_ARCHIVE"
        return
    fi

    require_command curl
    local tmp="$BOOTSTRAP_ARCHIVE.tmp.$$"
    rm -f "$tmp"
    log "Downloading $BOOTSTRAP_URL"
    curl -L --fail --progress-bar "$BOOTSTRAP_URL" -o "$tmp"
    local actual
    actual="$(sha256_of "$tmp")"
    [ "$actual" = "$BOOTSTRAP_SHA256" ] || {
        rm -f "$tmp"
        die "SHA-256 mismatch: expected $BOOTSTRAP_SHA256, got $actual"
    }
    mv "$tmp" "$BOOTSTRAP_ARCHIVE"
}

download_arch_repo_file() {
    local rel="$1"
    local expected="$2"
    local dst="$3"
    local url="$ARCH_REPO_BASE_URL/$rel"
    mkdir -p "$(dirname "$dst")"
    if [ -f "$dst" ]; then
        local actual
        actual="$(sha256_of "$dst")"
        if [ "$actual" = "$expected" ]; then
            return
        fi
        rm -f "$dst"
    fi

    require_command curl
    local tmp="$dst.tmp.$$"
    rm -f "$tmp"
    log "Downloading $url"
    curl -L --fail --progress-bar "$url" -o "$tmp"
    local actual
    actual="$(sha256_of "$tmp")"
    [ "$actual" = "$expected" ] || {
        rm -f "$tmp"
        die "SHA-256 mismatch for $url: expected $expected, got $actual"
    }
    mv "$tmp" "$dst"
}

download_arch_offline_repo_artifact() {
    local rel="$1"
    local expected="$2"
    local dst="$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/$rel"
    download_arch_repo_file "$rel" "$expected" "$dst"
}

strip_arch_repo_desc_for_guest() {
    awk '
        /^%SHA256SUM%$/ || /^%PGPSIG%$/ { skip = 1; next }
        /^%[A-Z0-9_]+%$/ { skip = 0 }
        !skip { print }
    '
}

stage_arch_minimal_repo_db() {
    local rel="$1"
    local upstream_sha="$2"
    local final_sha="$3"
    shift 3
    local dst="$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/$rel"
    if [ -f "$dst" ]; then
        local actual
        actual="$(sha256_of "$dst")"
        if [ "$actual" = "$final_sha" ]; then
            return
        fi
        rm -f "$dst"
    fi

    require_command tar
    require_command gzip
    local upstream="$CACHE/arch-repo/$ARCH_REPO_SNAPSHOT/$rel"
    download_arch_repo_file "$rel" "$upstream_sha" "$upstream"

    local work="$TARGET/.pacman-db-${rel//\//_}-$$"
    safe_clean_dir "$work"
    mkdir -p "$work"
    local pkg
    local paths=()
    for pkg in "$@"; do
        mkdir -p "$work/$pkg"
        tar -xOf "$upstream" "$pkg/desc" | strip_arch_repo_desc_for_guest > "$work/$pkg/desc" \
            || die "failed to extract $pkg/desc from $upstream"
        chmod 755 "$work/$pkg"
        chmod 644 "$work/$pkg/desc"
        paths+=("$pkg")
    done

    mkdir -p "$(dirname "$dst")"
    local tmp="$dst.tmp.$$"
    rm -f "$tmp"
    LC_ALL=C tar --format=ustar --sort=name --owner=0 --group=0 --numeric-owner \
        --mtime='1970-01-01 00:00Z' -C "$work" -cf - "${paths[@]}" \
        | gzip -n > "$tmp"
    local actual
    actual="$(sha256_of "$tmp")"
    [ "$actual" = "$final_sha" ] || {
        rm -f "$tmp"
        safe_clean_dir "$work"
        die "SHA-256 mismatch for generated $rel: expected $final_sha, got $actual"
    }
    mv "$tmp" "$dst"
    safe_clean_dir "$work"
}

stage_arch_pacman_package_cache() {
    local cache_dir="$ARCH_ROOTFS/var/cache/pacman/pkg"
    mkdir -p "$cache_dir"
    local entry rel base
    for entry in "${ARCH_OFFLINE_REPO_PACKAGES[@]}"; do
        rel="${entry%%:*}"
        base="$(basename "$rel")"
        rm -f "$cache_dir/$base" "$cache_dir/$base.part" "$cache_dir/$base.sig"
    done
}

stage_arch_pacman_package_aliases() {
    mkdir -p "$ARCH_ROOTFS/p"
    local entry rel target
    for entry in "${ARCH_OFFLINE_REPO_PACKAGE_ALIASES[@]}"; do
        rel="${entry%%:*}"
        target="${entry#*:}"
        ln -sfn "$target" "$ARCH_ROOTFS/$rel"
    done
}

stage_arch_offline_pacman_repo() {
    download_arch_offline_repo_artifact \
        "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst" \
        "95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst" \
        "96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst" \
        "f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    stage_arch_minimal_repo_db \
        "core/os/x86_64/core.db" \
        "45037c1a6abb70a08cd225f1f2e98f6f1a0140117eba54a24843b581bf884a56" \
        "a7e2c5c084e9bf7db0a5c9231942a4bd12cf170b85ae8d048e10d75acbe74e4d" \
        "gpm-1.20.7.r38.ge82d1a6-6"
    stage_arch_minimal_repo_db \
        "extra/os/x86_64/extra.db" \
        "2c4b923190d67f414ee981a020ca00a9f46c0e4ac44efa33fc067e2369e0387d" \
        "2b5a0cb4a6e5503a060c6011fb6cacb58c3b990a44829c3b70646641117b94c4" \
        "vim-9.2.0573-1" \
        "vim-runtime-9.2.0573-1"
    stage_arch_pacman_sync_dbs
    stage_arch_pacman_package_cache
    stage_arch_pacman_package_aliases
}

stage_arch_pacman_sync_dbs() {
    mkdir -p "$ARCH_ROOTFS/var/lib/pacman/sync"
    cp "$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/core/os/x86_64/core.db" \
        "$ARCH_ROOTFS/var/lib/pacman/sync/core.db"
    cp "$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/extra/os/x86_64/extra.db" \
        "$ARCH_ROOTFS/var/lib/pacman/sync/extra.db"
}

extract_bootstrap() {
    if [ "${LUPOS_ARCH_REFRESH:-0}" != "1" ] && rootfs_ready; then
        log "Arch rootfs already present at $ARCH_ROOTFS"
        make_rootfs_user_writable
        return
    fi

    require_command tar
    mkdir -p "$TARGET"
    local tmp="$TARGET/.extract-$$"
    safe_clean_dir "$tmp"
    mkdir -p "$tmp"
    log "Extracting bootstrap to $ARCH_ROOTFS"
    tar --warning=no-unknown-keyword --no-same-owner --delay-directory-restore \
        -xpf "$BOOTSTRAP_ARCHIVE" -C "$tmp"
    [ -d "$tmp/root.x86_64" ] || die "tarball did not contain root.x86_64/"
    safe_clean_dir "$ARCH_ROOTFS"
    mv "$tmp/root.x86_64" "$ARCH_ROOTFS"
    safe_clean_dir "$tmp"
    make_rootfs_user_writable
}

write_file() {
    local path="$1"
    mkdir -p "$(dirname "$path")"
    cat > "$path"
}

normalize_arch_pam() {
    local system_login="$ARCH_ROOTFS/etc/pam.d/system-login"
    [ -f "$system_login" ] || return
    sed -i 's/^-*session[[:space:]]\+optional[[:space:]]\+pam_systemd\.so/session optional pam_systemd.so/' "$system_login"
}

normalize_arch_pacman() {
    local conf="$ARCH_ROOTFS/etc/pacman.conf"
    [ -f "$conf" ] || return
    sed -i \
        -e 's/^[[:space:]]*DownloadUser[[:space:]]*=.*/#DownloadUser = alpm/' \
        -e 's/^[#[:space:]]*DisableSandboxFilesystem.*/#DisableSandboxFilesystem/' \
        -e 's/^[#[:space:]]*DisableSandboxSyscalls.*/#DisableSandboxSyscalls/' \
        -e '/^[[:space:]]*XferCommand[[:space:]]*=/d' \
        "$conf"
    if ! grep -Fxq "$ARCH_PACMAN_XFER_COMMAND" "$conf"; then
        if grep -Eq '^[#[:space:]]*Architecture[[:space:]]*=' "$conf"; then
            sed -i "/^[#[:space:]]*Architecture[[:space:]]*=/a $ARCH_PACMAN_XFER_COMMAND" "$conf"
        else
            sed -i "/^\\[options\\]$/a $ARCH_PACMAN_XFER_COMMAND" "$conf"
        fi
    fi
    if ! grep -Eq '^[[:space:]]*DisableSandbox[[:space:]]*$' "$conf"; then
        sed -i '/^[#[:space:]]*DisableSandboxSyscalls/a DisableSandbox' "$conf"
    fi
    for repo in core extra; do
        sed -i "/^\\[$repo\\]$/,/^\\[/ { /^${ARCH_PACMAN_SERVER//\//\\/}$/d; }" "$conf"
        if ! pacman_repo_siglevel_ready "$conf" "$repo"; then
            sed -i "/^\\[$repo\\]$/a SigLevel = Optional TrustAll" "$conf"
        fi
        if ! pacman_repo_server_ready "$conf" "$repo"; then
            sed -i "/^\\[$repo\\]$/a $ARCH_PACMAN_SERVER" "$conf"
        fi
    done
}

apply_lupos_overlay() {
    local S="$ARCH_ROOTFS"
    log "Applying Lupos overlay"

    write_file "$S/etc/hostname" <<< "lupos"
    write_file "$S/etc/hosts" <<'EOF'
127.0.0.1 localhost lupos
::1 localhost
EOF
    write_file "$S/etc/resolv.conf" <<< "nameserver 10.0.2.3"

    mkdir -p "$S/etc/pacman.d"
    write_file "$S/etc/pacman.d/mirrorlist" <<'EOF'
## Lupos stages a pinned Arch Archive subset locally because the guest does
## not yet provide TCP networking for libalpm downloads.
Server = file:///var/lib/lupos/pacman-repo/$repo/os/$arch
EOF
    write_file "$S/$ARCH_PACMAN_XFER_HELPER" <<'EOF'
#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: lupos-pacman-xfer URL OUTPUT" >&2
    exit 2
fi

src=$1
dst=$2

case "$src" in
    file://*) src=${src#file://} ;;
    *)
        echo "unsupported pacman transfer URL: $src" >&2
        exit 2
        ;;
esac

base=${src##*/}
alias=
case "$base" in
    gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst) alias=/p/g ;;
    vim-9.2.0573-1-x86_64.pkg.tar.zst) alias=/p/v ;;
    vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst) alias=/p/r ;;
esac

if [ -n "$alias" ]; then
    rm -f "$dst"
    ln -s "$alias" "$dst"
else
    cp "$src" "$dst"
fi
EOF
    chmod 755 "$S/$ARCH_PACMAN_XFER_HELPER"
    stage_arch_offline_pacman_repo

    write_file "$S/etc/fstab" <<'EOF'
proc      /proc     proc      defaults 0 0
sysfs     /sys      sysfs     defaults 0 0
devtmpfs  /dev      devtmpfs  defaults 0 0
tmpfs     /tmp      tmpfs     defaults 0 0
tmpfs     /run      tmpfs     defaults 0 0
/swapfile none      swap      sw       0 0
EOF

    write_file "$S/.lupos-profile" <<'EOF'
distro=arch
builder=lupos-unprivileged-bootstrap
EOF

    mkdir -p "$S/etc/systemd/network"
    write_file "$S/etc/systemd/network/10-lupos-qemu.network" <<'EOF'
[Match]
Name=e* eth* en*

[Network]
Address=10.0.2.15/24
Gateway=10.0.2.2
DNS=10.0.2.3
EOF

    mkdir -p \
        "$S/etc/systemd/system/getty.target.wants" \
        "$S/etc/systemd/system/getty@tty1.service.d" \
        "$S/etc/systemd/system/multi-user.target.wants"

    ln -sfn /usr/lib/systemd/system/multi-user.target \
        "$S/etc/systemd/system/default.target"
    ln -sfn /usr/lib/systemd/system/getty@.service \
        "$S/etc/systemd/system/getty.target.wants/getty@tty1.service"

    write_file "$S/etc/systemd/system/getty@tty1.service.d/lupos.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --noclear --nohostname tty1 linux
EOF

    for svc in systemd-networkd systemd-resolved; do
        [ -f "$S/usr/lib/systemd/system/${svc}.service" ] || continue
        ln -sfn "/usr/lib/systemd/system/${svc}.service" \
            "$S/etc/systemd/system/multi-user.target.wants/${svc}.service"
    done

    normalize_arch_pam
    normalize_arch_pacman
}

copy_to_stage() {
    log "Staging into $STAGE"
    mkdir -p "$(dirname "$STAGE")"
    safe_clean_dir "$STAGE"
    mkdir -p "$STAGE"
    cp -a "$ARCH_ROOTFS/." "$STAGE/"
}

validate_stage() {
    local bad=0
    for p in \
        usr/lib/systemd/systemd \
        usr/bin/bash \
        usr/bin/pacman \
        etc/os-release \
        etc/pacman.conf \
        "$ARCH_PACMAN_XFER_HELPER" \
        etc/systemd/network/10-lupos-qemu.network \
        var/lib/pacman/local/ALPM_DB_VERSION \
        var/lib/pacman/sync/core.db \
        var/lib/pacman/sync/extra.db
    do
        [ -e "$STAGE/$p" ] && continue
        echo "error: missing stage artifact: $STAGE/$p" >&2
        bad=1
    done
    [ "$bad" = "0" ] || exit 1
    pacman_mirrorlist_ready "$STAGE" || die "staged pacman mirrorlist has no active Server entries"
    pacman_config_ready "$STAGE" || die "staged pacman config must disable sandboxing, define direct offline repo servers, use the Lupos transfer helper, and avoid DownloadUser"
    pacman_offline_repo_ready "$STAGE" || die "staged offline pacman repo is missing pinned database/package files or preseeded sync databases"
    pam_systemd_ready "$STAGE" || die "staged PAM systemd session hook is missing"
    : > "$STAGE_STAMP"
}

main() {
    ensure_unprivileged
    [ "$LUPOS_DISTRO" = "arch" ] || die "build_userland.sh now stages Arch; got LUPOS_DISTRO=$LUPOS_DISTRO"
    [ "$(uname -s)" = "Linux" ] || die "Arch bootstrap requires a Linux host"

    if [ "${LUPOS_ARCH_REFRESH:-0}" != "1" ] && stage_ready; then
        log "Stage already present at $STAGE (set LUPOS_ARCH_REFRESH=1 to rebuild)"
        return
    fi

    download_bootstrap
    extract_bootstrap
    apply_lupos_overlay
    copy_to_stage
    validate_stage
    log "Done - $STAGE"
}

main "$@"
