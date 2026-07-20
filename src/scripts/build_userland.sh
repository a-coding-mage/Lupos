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
LUPOS_USERLAND_GRAPHICS="${LUPOS_USERLAND_GRAPHICS:-0}"
BOOTSTRAP_NAME="${LUPOS_ARCH_BOOTSTRAP_NAME:-archlinux-bootstrap-2026.06.01-x86_64}"
BOOTSTRAP_SHA256="${LUPOS_ARCH_BOOTSTRAP_SHA256:-e68ba918c9f7deede8eccd2cd8ce259df104d84b0791cff3a2bc7579ced34849}"
BOOTSTRAP_URL="${LUPOS_ARCH_BOOTSTRAP_URL:-https://archive.archlinux.org/iso/2026.06.01/${BOOTSTRAP_NAME}.tar.zst}"
ARCH_REPO_SNAPSHOT="2026/06/01"
ARCH_REPO_BASE_URL="${LUPOS_ARCH_REPO_BASE_URL:-https://archive.archlinux.org/repos/$ARCH_REPO_SNAPSHOT}"
ARCH_OFFLINE_REPO_REL="var/lib/lupos/pacman-repo"
ARCH_PACMAN_SERVER='Server = file:///var/lib/lupos/pacman-repo/$repo/os/$arch'
ARCH_PACMAN_ARCHIVE_SERVER='Server = https://archive.archlinux.org/repos/2026/06/01/$repo/os/$arch'
ARCH_PACMAN_CONFIG="etc/pacman-lupos.conf"
ARCH_PACMAN_XFER_HELPER="usr/lib/lupos/pacman-xfer"
ARCH_PACMAN_XFER_COMMAND="XferCommand = /usr/lib/lupos/pacman-xfer %u %o"
ARCH_PACMAN_GPG_DIR="etc/pacman.d/gnupg"
ARCH_SYSTEMD_HOOK_SCRIPT="usr/share/libalpm/scripts/systemd-hook"
ARCH_OFFLINE_REPO_ARTIFACTS=(
    "core/os/x86_64/core.db:45037c1a6abb70a08cd225f1f2e98f6f1a0140117eba54a24843b581bf884a56"
    "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst:95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    "core/os/x86_64/nano-9.0-1-x86_64.pkg.tar.zst:2b52fdaafc70e63511f3b85d98c34f26151b0498fa6f0e61fcb4be4b0d754edf"
    "extra/os/x86_64/extra.db:2c4b923190d67f414ee981a020ca00a9f46c0e4ac44efa33fc067e2369e0387d"
    "extra/os/x86_64/fastfetch-2.63.1-1-x86_64.pkg.tar.zst:d36690ae3a1c342da660ac499655eba532b0ceab58c976b92d943892c1ed232e"
    "extra/os/x86_64/fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst:64dcc7ccaa5460b93ce1c76a9e104bcbb373d3d15fb5abc01a1192f4607e9d2e"
    "extra/os/x86_64/freetype2-2.14.3-1-x86_64.pkg.tar.zst:fcaa410420dea42779d02aa76f1cc95d8430bdc52071ac6219d33306899b8655"
    "extra/os/x86_64/libice-1.1.2-1-x86_64.pkg.tar.zst:bb613be39e5bc1707a39f895c178674bbda52f022da51612e7a10a386608e107"
    "extra/os/x86_64/libpng-1.6.58-1-x86_64.pkg.tar.zst:8d80045f15f6339b2284db5df06a4287225cc2389c24dbb2fbc23458f6887ee5"
    "extra/os/x86_64/libsm-1.2.6-1-x86_64.pkg.tar.zst:ce334b07a9701ba6ef4d610257500f7b035dc1cdc0a658e056d293547cde0976"
    "extra/os/x86_64/libutempter-1.2.3-1-x86_64.pkg.tar.zst:e1dd15aed4cd76b42729d5dc58d4db3dd49d6c5fe08b33abee0df3bcfc98fd13"
    "extra/os/x86_64/libx11-1.8.13-1-x86_64.pkg.tar.zst:251f58e0a9bc2cd69b8e708f239914e48e3a91cd1822220806d273be873d026f"
    "extra/os/x86_64/libxau-1.0.12-1-x86_64.pkg.tar.zst:605c8b059c36792f4e0cc235acadf39d0762df6c7878825a1be01a00ae7ea21e"
    "extra/os/x86_64/libxaw-1.0.16-2-x86_64.pkg.tar.zst:79966ba7df3cf46bbcac4292384247b0070bfd036f3a55dbcd9121cb4ccf8a38"
    "extra/os/x86_64/libxcb-1.17.0-1-x86_64.pkg.tar.zst:2b2e7ac64b1d56c08a227c10bcab179605f2773f31db0a8c89f49f4e5b2f1292"
    "extra/os/x86_64/libxdmcp-1.1.5-2-x86_64.pkg.tar.zst:623c957c2fd4b427a0f5a531da44931f9f66521391ee0bd0e635479947036b65"
    "extra/os/x86_64/libxext-1.3.7-1-x86_64.pkg.tar.zst:ac56905dc51bb652eca5f706fd7e7bb7ea81d4e057a236139fc769ce5ea10cf1"
    "extra/os/x86_64/libxft-2.3.9-1-x86_64.pkg.tar.zst:a7841ed8e67dc3f94eca4672c8961329dfdd6b67836f8ac720a246d3aab02ecb"
    "extra/os/x86_64/libxmu-1.3.1-1-x86_64.pkg.tar.zst:31870eb5ad1911880cd7b2f3a59292b644d84b873ed62a1a7c36b9ac26bc08a5"
    "extra/os/x86_64/libxpm-3.5.19-1-x86_64.pkg.tar.zst:cb4df58d300485410132dcd0deeb209c0ef9a6e8d5280ec0f4cb532aba2c208e"
    "extra/os/x86_64/libxrender-0.9.12-1-x86_64.pkg.tar.zst:fed0389073d5b107074eaab48cefcc2716e607865142cde5b579c8ceeefea142"
    "extra/os/x86_64/libxt-1.3.1-1-x86_64.pkg.tar.zst:5d4ee1f73c946cdd9b908b127157d6d826d198be982c91e199daf6bdd7f6b9ec"
    "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst:f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst:96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
    "extra/os/x86_64/xcb-proto-1.17.0-4-any.pkg.tar.zst:98f661ef7c7e05eb7a687e859cf123a92806ceed8f55c0d70ddbac799988239a"
    "extra/os/x86_64/xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst:df9e0d1f29ae0135a49b17157841a4cc7c97869f9f27fd7480fff64f9afb94b7"
    "extra/os/x86_64/xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst:edf4f98e4c787074b3d182daf9eadfacee3ce14785397ccd63fe11f9ffb1c903"
    "extra/os/x86_64/xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst:b80ea53e86c02b3c7697c8353ccc41df80278b57f0134f6a608be126a0724bb3"
    "extra/os/x86_64/xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst:e916cb35a6a3031ddd6c6d49d795bcd702dcc532debb911dc97cec715a235c30"
    "extra/os/x86_64/xorgproto-2025.1-1-any.pkg.tar.zst:f7bf3eed570618511fb53cc1bd32c2f1ee82e02662075436a59e6e50436f30de"
    "extra/os/x86_64/xterm-410-1-x86_64.pkg.tar.zst:85b31fbadc47676215007d2fb08115a237e30c385e382d98f7cf6c59a77d9fa9"
    "extra/os/x86_64/yyjson-0.12.0-1-x86_64.pkg.tar.zst:a25b2c4be6039c36ef9a7f440ac984772e5052aacac24f6046757053c7d77b58"
)
ARCH_OFFLINE_REPO_PACKAGES=(
    "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst:95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    "core/os/x86_64/nano-9.0-1-x86_64.pkg.tar.zst:2b52fdaafc70e63511f3b85d98c34f26151b0498fa6f0e61fcb4be4b0d754edf"
    "extra/os/x86_64/fastfetch-2.63.1-1-x86_64.pkg.tar.zst:d36690ae3a1c342da660ac499655eba532b0ceab58c976b92d943892c1ed232e"
    "extra/os/x86_64/fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst:64dcc7ccaa5460b93ce1c76a9e104bcbb373d3d15fb5abc01a1192f4607e9d2e"
    "extra/os/x86_64/freetype2-2.14.3-1-x86_64.pkg.tar.zst:fcaa410420dea42779d02aa76f1cc95d8430bdc52071ac6219d33306899b8655"
    "extra/os/x86_64/libice-1.1.2-1-x86_64.pkg.tar.zst:bb613be39e5bc1707a39f895c178674bbda52f022da51612e7a10a386608e107"
    "extra/os/x86_64/libpng-1.6.58-1-x86_64.pkg.tar.zst:8d80045f15f6339b2284db5df06a4287225cc2389c24dbb2fbc23458f6887ee5"
    "extra/os/x86_64/libsm-1.2.6-1-x86_64.pkg.tar.zst:ce334b07a9701ba6ef4d610257500f7b035dc1cdc0a658e056d293547cde0976"
    "extra/os/x86_64/libutempter-1.2.3-1-x86_64.pkg.tar.zst:e1dd15aed4cd76b42729d5dc58d4db3dd49d6c5fe08b33abee0df3bcfc98fd13"
    "extra/os/x86_64/libx11-1.8.13-1-x86_64.pkg.tar.zst:251f58e0a9bc2cd69b8e708f239914e48e3a91cd1822220806d273be873d026f"
    "extra/os/x86_64/libxau-1.0.12-1-x86_64.pkg.tar.zst:605c8b059c36792f4e0cc235acadf39d0762df6c7878825a1be01a00ae7ea21e"
    "extra/os/x86_64/libxaw-1.0.16-2-x86_64.pkg.tar.zst:79966ba7df3cf46bbcac4292384247b0070bfd036f3a55dbcd9121cb4ccf8a38"
    "extra/os/x86_64/libxcb-1.17.0-1-x86_64.pkg.tar.zst:2b2e7ac64b1d56c08a227c10bcab179605f2773f31db0a8c89f49f4e5b2f1292"
    "extra/os/x86_64/libxdmcp-1.1.5-2-x86_64.pkg.tar.zst:623c957c2fd4b427a0f5a531da44931f9f66521391ee0bd0e635479947036b65"
    "extra/os/x86_64/libxext-1.3.7-1-x86_64.pkg.tar.zst:ac56905dc51bb652eca5f706fd7e7bb7ea81d4e057a236139fc769ce5ea10cf1"
    "extra/os/x86_64/libxft-2.3.9-1-x86_64.pkg.tar.zst:a7841ed8e67dc3f94eca4672c8961329dfdd6b67836f8ac720a246d3aab02ecb"
    "extra/os/x86_64/libxmu-1.3.1-1-x86_64.pkg.tar.zst:31870eb5ad1911880cd7b2f3a59292b644d84b873ed62a1a7c36b9ac26bc08a5"
    "extra/os/x86_64/libxpm-3.5.19-1-x86_64.pkg.tar.zst:cb4df58d300485410132dcd0deeb209c0ef9a6e8d5280ec0f4cb532aba2c208e"
    "extra/os/x86_64/libxrender-0.9.12-1-x86_64.pkg.tar.zst:fed0389073d5b107074eaab48cefcc2716e607865142cde5b579c8ceeefea142"
    "extra/os/x86_64/libxt-1.3.1-1-x86_64.pkg.tar.zst:5d4ee1f73c946cdd9b908b127157d6d826d198be982c91e199daf6bdd7f6b9ec"
    "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst:f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst:96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
    "extra/os/x86_64/xcb-proto-1.17.0-4-any.pkg.tar.zst:98f661ef7c7e05eb7a687e859cf123a92806ceed8f55c0d70ddbac799988239a"
    "extra/os/x86_64/xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst:df9e0d1f29ae0135a49b17157841a4cc7c97869f9f27fd7480fff64f9afb94b7"
    "extra/os/x86_64/xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst:edf4f98e4c787074b3d182daf9eadfacee3ce14785397ccd63fe11f9ffb1c903"
    "extra/os/x86_64/xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst:b80ea53e86c02b3c7697c8353ccc41df80278b57f0134f6a608be126a0724bb3"
    "extra/os/x86_64/xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst:e916cb35a6a3031ddd6c6d49d795bcd702dcc532debb911dc97cec715a235c30"
    "extra/os/x86_64/xorgproto-2025.1-1-any.pkg.tar.zst:f7bf3eed570618511fb53cc1bd32c2f1ee82e02662075436a59e6e50436f30de"
    "extra/os/x86_64/xterm-410-1-x86_64.pkg.tar.zst:85b31fbadc47676215007d2fb08115a237e30c385e382d98f7cf6c59a77d9fa9"
    "extra/os/x86_64/yyjson-0.12.0-1-x86_64.pkg.tar.zst:a25b2c4be6039c36ef9a7f440ac984772e5052aacac24f6046757053c7d77b58"
)
ARCH_OFFLINE_REPO_SIGNATURES=(
    "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst.sig:70214ba008476ed6457ff52c61e3a9750780a2967012680dc822d139521d7868"
    "core/os/x86_64/nano-9.0-1-x86_64.pkg.tar.zst.sig:da26270e9831bf5495dd697b219aac57cabfe2c70b2a439605021c6f1032a30c"
    "extra/os/x86_64/fastfetch-2.63.1-1-x86_64.pkg.tar.zst.sig:2e9e193b614553463e4ea7fdcef8423cbf627f3a5c99b8b14812c775c3742f54"
    "extra/os/x86_64/fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst.sig:a4846d9d359bb0fc07339dca4441fa77c3d45f2ff411dd21b2c382a63daddd28"
    "extra/os/x86_64/freetype2-2.14.3-1-x86_64.pkg.tar.zst.sig:ba2b4c9291b8187e9c077bac0df7253c6d34e0330d75bf0bf164dd0ddbdd9732"
    "extra/os/x86_64/libice-1.1.2-1-x86_64.pkg.tar.zst.sig:61bc66967145883780a56a1eb8c726e9cdf8b9c74ee984efd9a5d4d6e1682c62"
    "extra/os/x86_64/libpng-1.6.58-1-x86_64.pkg.tar.zst.sig:ee04d806dfd4a506b685059f2dfc92faad3f60886dcbf9b18e0f092109dbbd18"
    "extra/os/x86_64/libsm-1.2.6-1-x86_64.pkg.tar.zst.sig:51b51bfb204de1202336b1b901da4a9d33b6ef57d3dc458bc408a6154d32c01a"
    "extra/os/x86_64/libutempter-1.2.3-1-x86_64.pkg.tar.zst.sig:fba77cbdcffc70408f3d825949116160a27b454ca4ad7295b15019c121bee735"
    "extra/os/x86_64/libx11-1.8.13-1-x86_64.pkg.tar.zst.sig:2797bf6bba9b3cdc09026790ab349e4b6930ef31c0cdde87306dea3b48f1794c"
    "extra/os/x86_64/libxau-1.0.12-1-x86_64.pkg.tar.zst.sig:d0fc4f60d3a25addaf7ce78b5d7eb5fe5d46b6c6a3086e6946dbf98b0ecead77"
    "extra/os/x86_64/libxaw-1.0.16-2-x86_64.pkg.tar.zst.sig:ffa32389e3a415f9c5b83d2ad979e263d201317b03c7b7cad6736238ad95baa3"
    "extra/os/x86_64/libxcb-1.17.0-1-x86_64.pkg.tar.zst.sig:e6a8cef99027f32595301710a49d9e3a1fb0ed7284ba1e1da0150d98c9e6889a"
    "extra/os/x86_64/libxdmcp-1.1.5-2-x86_64.pkg.tar.zst.sig:cf2b3f2bd0f4c0329c3fa929304bf9566bb7d935f994baa0fda6b2aba507b328"
    "extra/os/x86_64/libxext-1.3.7-1-x86_64.pkg.tar.zst.sig:1a834468210368faff1b121a45f51f14bb37d6ed92a34737f238a909cdb3d840"
    "extra/os/x86_64/libxft-2.3.9-1-x86_64.pkg.tar.zst.sig:dc6d742b26b5a34718c15981967cda7af61f233a7565b96dbad2d3cfd75f8786"
    "extra/os/x86_64/libxmu-1.3.1-1-x86_64.pkg.tar.zst.sig:b0939e8d418055df91895aebfdf5fdbc6df1b93cea0283b4750e3f836c371699"
    "extra/os/x86_64/libxpm-3.5.19-1-x86_64.pkg.tar.zst.sig:d5294ade5d79ec4b24989181ee3308f2299f6a4a1e2b9be84aad00bf000d91ae"
    "extra/os/x86_64/libxrender-0.9.12-1-x86_64.pkg.tar.zst.sig:0bba31c86ddc3ec7ac336e52e3dbe94b8cb6779837b6ecf1fa9adb54613ff2e7"
    "extra/os/x86_64/libxt-1.3.1-1-x86_64.pkg.tar.zst.sig:dea7f540d148afbe390608d147f2e568b75a19d8c1b4c5bce70d0bda0db1b1b5"
    "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst.sig:2a137c9157af719429b1f704e39a3137b1bc1a4b698831ae6994e69602745868"
    "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst.sig:25e823c22875673041b1fc8c45b0bd0434a45bbd9702085f87f472f491d1138d"
    "extra/os/x86_64/xcb-proto-1.17.0-4-any.pkg.tar.zst.sig:b0e77b6b0b4c4fa602ae0ad378b87ffbc4dd82a2d3e1d3686a19b041cf113200"
    "extra/os/x86_64/xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst.sig:6643971afb233adb4e2bc84309fe4b58c7b3b0d6282db4f2852f440696119868"
    "extra/os/x86_64/xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst.sig:34101d8ddef1aadf437e3f8bc82a9e5680414bcb25df6f87a13968a987540662"
    "extra/os/x86_64/xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst.sig:b84ce8172bd3404f600066796cc96e43bfa335ab77bfd60529df05ea70872df4"
    "extra/os/x86_64/xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst.sig:7a68065ce7852ca9377164e2132453f1435315fe423dcd3a5d418ab62f5b00b2"
    "extra/os/x86_64/xorgproto-2025.1-1-any.pkg.tar.zst.sig:1c084076fcdbac01a098e9d1639a0d20eb0561e655a3d06f46a6a74e04ec0223"
    "extra/os/x86_64/xterm-410-1-x86_64.pkg.tar.zst.sig:3a5d6d0c2fcc9550e20e62fdf273b7ecf0f1995e0d1296ca7c12fdd1a6295249"
    "extra/os/x86_64/yyjson-0.12.0-1-x86_64.pkg.tar.zst.sig:3308dd111fac8765ae9bec75a82aa18e6efe108a600962cca46b82a61a375bb2"
)
ARCH_OFFLINE_REPO_PACKAGE_ALIASES=(
    "p/fastfetch:/var/lib/lupos/pacman-repo/extra/os/x86_64/fastfetch-2.63.1-1-x86_64.pkg.tar.zst"
    "p/fontconfig:/var/lib/lupos/pacman-repo/extra/os/x86_64/fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst"
    "p/freetype2:/var/lib/lupos/pacman-repo/extra/os/x86_64/freetype2-2.14.3-1-x86_64.pkg.tar.zst"
    "p/g:/var/lib/lupos/pacman-repo/core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst"
    "p/libice:/var/lib/lupos/pacman-repo/extra/os/x86_64/libice-1.1.2-1-x86_64.pkg.tar.zst"
    "p/libpng:/var/lib/lupos/pacman-repo/extra/os/x86_64/libpng-1.6.58-1-x86_64.pkg.tar.zst"
    "p/libsm:/var/lib/lupos/pacman-repo/extra/os/x86_64/libsm-1.2.6-1-x86_64.pkg.tar.zst"
    "p/libutempter:/var/lib/lupos/pacman-repo/extra/os/x86_64/libutempter-1.2.3-1-x86_64.pkg.tar.zst"
    "p/libx11:/var/lib/lupos/pacman-repo/extra/os/x86_64/libx11-1.8.13-1-x86_64.pkg.tar.zst"
    "p/libxau:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxau-1.0.12-1-x86_64.pkg.tar.zst"
    "p/libxaw:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxaw-1.0.16-2-x86_64.pkg.tar.zst"
    "p/libxcb:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxcb-1.17.0-1-x86_64.pkg.tar.zst"
    "p/libxdmcp:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxdmcp-1.1.5-2-x86_64.pkg.tar.zst"
    "p/libxext:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxext-1.3.7-1-x86_64.pkg.tar.zst"
    "p/libxft:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxft-2.3.9-1-x86_64.pkg.tar.zst"
    "p/libxmu:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxmu-1.3.1-1-x86_64.pkg.tar.zst"
    "p/libxpm:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxpm-3.5.19-1-x86_64.pkg.tar.zst"
    "p/libxrender:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxrender-0.9.12-1-x86_64.pkg.tar.zst"
    "p/libxt:/var/lib/lupos/pacman-repo/extra/os/x86_64/libxt-1.3.1-1-x86_64.pkg.tar.zst"
    "p/n:/var/lib/lupos/pacman-repo/core/os/x86_64/nano-9.0-1-x86_64.pkg.tar.zst"
    "p/v:/var/lib/lupos/pacman-repo/extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst"
    "p/r:/var/lib/lupos/pacman-repo/extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst"
    "p/xcb-proto:/var/lib/lupos/pacman-repo/extra/os/x86_64/xcb-proto-1.17.0-4-any.pkg.tar.zst"
    "p/xorg-xauth:/var/lib/lupos/pacman-repo/extra/os/x86_64/xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst"
    "p/xorg-xinit:/var/lib/lupos/pacman-repo/extra/os/x86_64/xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst"
    "p/xorg-xmodmap:/var/lib/lupos/pacman-repo/extra/os/x86_64/xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst"
    "p/xorg-xrdb:/var/lib/lupos/pacman-repo/extra/os/x86_64/xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst"
    "p/xorgproto:/var/lib/lupos/pacman-repo/extra/os/x86_64/xorgproto-2025.1-1-any.pkg.tar.zst"
    "p/xterm:/var/lib/lupos/pacman-repo/extra/os/x86_64/xterm-410-1-x86_64.pkg.tar.zst"
    "p/yyjson:/var/lib/lupos/pacman-repo/extra/os/x86_64/yyjson-0.12.0-1-x86_64.pkg.tar.zst"
)
ARCH_GRAPHICS_PACKAGES=(
    # The desktop exposes a normal multi-user shell, so ship the standard
    # privilege boundary instead of leaving `sudo` absent from PATH.
    sudo
    xorg-server
    xf86-video-fbdev
    xf86-input-evdev
    xorg-xinit
    xorg-twm
    xterm
    xorg-fonts-misc
    # Modern graphical login.  The GTK greeter reuses the GTK3/theme stack
    # already required by XFCE and avoids the classic Xaw xdm login widget.
    lightdm
    lightdm-gtk-greeter
    # XFCE desktop session (task: land in a real desktop, not twm).  pacman
    # resolves the full GTK3/xfconf/garcon dependency closure from the Arch
    # snapshot mirror automatically.
    xfce4-session
    xfwm4
    xfce4-panel
    # The unmodified stock panel's lower dock references
    # xfce4-appfinder.desktop, but xfce4-panel does not depend on the appfinder
    # package.  Install the referenced application instead of rewriting the
    # vendor panel configuration.
    xfce4-appfinder
    xfdesktop
    xfce4-settings
    thunar
    xfce4-terminal
    # A complete desktop media session: PipeWire provides the graph and ALSA /
    # PulseAudio compatibility endpoints, WirePlumber owns device policy, and
    # XFCE exposes both a panel volume control and the full pavucontrol mixer.
    alsa-utils
    pipewire
    pipewire-audio
    pipewire-alsa
    pipewire-pulse
    wireplumber
    pavucontrol
    xfce4-pulseaudio-plugin
    # Keep browser and standalone playback codec coverage explicit instead of
    # relying on Firefox's current transitive dependency closure.
    ffmpeg
    gst-libav
    gst-plugins-good
    gst-plugins-bad
    gst-plugins-ugly
    parole
    # XFCE's settings manager already supplies display controls.  NetworkManager
    # and its status applet add the corresponding live network settings UI.
    networkmanager
    network-manager-applet
    # Ship a usable terminal editor and browser in the image.  Installing them
    # in this host-side transaction avoids a large, slow first-boot pacman run.
    nano
    firefox
    # D-Bus for the per-session bus xfce4-session needs, plus a base icon theme
    # and scalable fonts so the panel/desktop render.
    dbus
    adwaita-icon-theme
    hicolor-icon-theme
    ttf-dejavu
    # Firefox suggestions follow the remote page/search locale and routinely
    # contain CJK text.  Bitmap xorg-fonts-misc fallback renders as missing or
    # garbled glyphs in modern Firefox, so ship a scalable CJK family.
    noto-fonts-cjk
)

TARGET="$ROOT/target/userland"
CACHE="$TARGET/cache"
BOOTSTRAP_ARCHIVE="$CACHE/${BOOTSTRAP_NAME}.tar.zst"
if [ -z "${LUPOS_ARCH_ROOTFS:-}" ]; then
    case "${LUPOS_USERLAND_GRAPHICS,,}" in
        1|true|yes|on) LUPOS_ARCH_ROOTFS="$TARGET/arch-graphics-rootfs" ;;
        *) LUPOS_ARCH_ROOTFS="$TARGET/arch-rootfs" ;;
    esac
fi
LUPOS_ARCH_BOOTSTRAP_ROOTFS="${LUPOS_ARCH_BOOTSTRAP_ROOTFS:-}"
ARCH_ROOTFS="${LUPOS_ARCH_BOOTSTRAP_ROOTFS:-$LUPOS_ARCH_ROOTFS}"
VENDOR_POLICY_STAMP="$ARCH_ROOTFS/.lupos-vendor-policy-v4"
VENDOR_MANIFEST_REL=".lupos-vendor-files-v1"
if [ -z "${STAGE:-}" ]; then
    case "${LUPOS_USERLAND_GRAPHICS,,}" in
        1|true|yes|on) STAGE="$TARGET/graphics-stage" ;;
        *) STAGE="$TARGET/stage" ;;
    esac
fi
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

graphics_enabled() {
    case "${LUPOS_USERLAND_GRAPHICS,,}" in
        1|true|yes|on) return 0 ;;
        *) return 1 ;;
    esac
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
        && [ -e "$ARCH_ROOTFS/var/lib/pacman/local/ALPM_DB_VERSION" ] \
        && [ -e "$VENDOR_POLICY_STAMP" ] \
        && [ -s "$ARCH_ROOTFS/$VENDOR_MANIFEST_REL" ]
}

make_rootfs_user_writable() {
    chmod -R u+rwX "$ARCH_ROOTFS"
}

pam_systemd_ready() {
    local r="$1"
    grep -Eq '^-session[[:space:]]+optional[[:space:]]+pam_systemd\.so$' "$r/etc/pam.d/system-login" \
        && grep -q 'pam_systemd\.so' "$r/usr/lib/pam.d/systemd-user"
}

pacman_repo_uses_default_siglevel() {
    local conf="$1"
    local repo="$2"
    awk -v repo="$repo" '
        $0 == "[" repo "]" { in_repo = 1; next }
        /^\[/ { in_repo = 0 }
        in_repo && /^[[:space:]]*SigLevel[[:space:]]*=/ { overridden = 1 }
        END { exit overridden ? 1 : 0 }
    ' "$conf"
}

pacman_keyring_ready() {
    local r="$1"
    local gpgdir="$r/$ARCH_PACMAN_GPG_DIR"
    [ -f "$gpgdir/pubring.gpg" ] \
        && [ -s "$gpgdir/pubring.kbx" ] \
        && [ -s "$gpgdir/trustdb.gpg" ] \
        && gpg --batch --no-permission-warning --homedir "$gpgdir" \
            --list-keys 2>/dev/null | grep -q '^pub'
}

pacman_repo_servers_ready() {
    local conf="$1"
    local repo="$2"
    awk \
        -v repo="$repo" \
        -v local_server="$ARCH_PACMAN_SERVER" \
        -v archive_server="$ARCH_PACMAN_ARCHIVE_SERVER" '
        $0 == "[" repo "]" { in_repo = 1; next }
        /^\[/ { in_repo = 0 }
        in_repo && $0 == local_server { local_found = 1; next }
        in_repo && local_found && $0 == archive_server { archive_after_local = 1 }
        END { exit local_found && archive_after_local ? 0 : 1 }
    ' "$conf"
}

pacman_config_ready() {
    local r="$1"
    local conf="$r/$ARCH_PACMAN_CONFIG"
    local helper="$r/$ARCH_PACMAN_XFER_HELPER"
    grep -Eq '^[[:space:]]*DisableSandbox[[:space:]]*$' "$conf" \
        && ! grep -Eq '^[[:space:]]*DownloadUser[[:space:]]*=' "$conf" \
        && grep -Fxq "$ARCH_PACMAN_XFER_COMMAND" "$conf" \
        && [ -x "$helper" ] \
        && [ -x "$r/usr/bin/curl" ] \
        && grep -Fq 'http://*|https://*)' "$helper" \
        && grep -Fq "exec /usr/bin/curl --disable --fail --location --proto '=http,https' --proto-redir '=http,https' --continue-at - --output \"\$dst\" \"\$src\"" "$helper" \
        && grep -Eq '^[[:space:]]*SigLevel[[:space:]]*=[[:space:]]*Required[[:space:]]+DatabaseOptional[[:space:]]*$' "$conf" \
        && pacman_repo_uses_default_siglevel "$conf" core \
        && pacman_repo_uses_default_siglevel "$conf" extra \
        && pacman_keyring_ready "$r" \
        && pacman_repo_servers_ready "$conf" core \
        && pacman_repo_servers_ready "$conf" extra
}

vendor_package_configs_pristine() {
    local r="$1"
    # pacman's stock configuration documents DisableSandbox and XferCommand in
    # comments.  Only active directives can indicate that the package-owned
    # file was repurposed for Lupos; the separate pacman-lupos.conf carries our
    # offline transport policy.
    ! grep -Eq '^[[:space:]]*DisableSandbox([[:alpha:]]+)?[[:space:]]*$|^[[:space:]]*XferCommand[[:space:]]*=[[:space:]]*/usr/lib/lupos' "$r/etc/pacman.conf" \
        && ! grep -Eq '^[[:space:]]*[^#[:space:]].*(lupos|/var/lib/lupos)' "$r/etc/pacman.conf" \
        && ! grep -Fq 'file:///var/lib/lupos/pacman-repo' "$r/etc/pacman.d/mirrorlist" \
        && ! grep -Fq 'disabled on Lupos' "$r/$ARCH_SYSTEMD_HOOK_SCRIPT"
}

write_vendor_files_manifest() {
    local r="$1"
    local out="$2"
    local rel path target
    local list="$TARGET/.vendor-files-list-$$"
    local rows="$TARGET/.vendor-files-rows-$$"
    local regulars="$TARGET/.vendor-files-regulars-$$"

    : > "$list"
    : > "$rows"
    : > "$regulars"
    awk '
        $0 == "%FILES%" { in_files = 1; next }
        in_files && /^%.*%$/ { in_files = 0 }
        in_files && length($0) { print }
    ' "$r"/var/lib/pacman/local/*/files | LC_ALL=C sort -u > "$list"

    while IFS= read -r rel; do
        path="$r/$rel"
        if [ -L "$path" ]; then
            target="$(readlink "$path")"
            printf 'link\t%s\t%s\n' "$rel" "$target" >> "$rows"
        elif [ -f "$path" ]; then
            printf '%s\0' "$path" >> "$regulars"
        elif [ -d "$path" ]; then
            printf 'dir\t%s\n' "$rel" >> "$rows"
        elif [ -e "$path" ]; then
            printf 'other\t%s\n' "$rel" >> "$rows"
        else
            printf 'missing\t%s\n' "$rel" >> "$rows"
        fi
    done < "$list"

    # Hash regular files in batches; spawning sha256sum once per package file
    # makes a full XFCE stage needlessly expensive.
    if [ -s "$regulars" ]; then
        xargs -0 -r sha256sum < "$regulars" | awk -v prefix="$r/" '
            {
                digest = $1
                path = substr($0, length($1) + 3)
                if (index(path, prefix) == 1)
                    path = substr(path, length(prefix) + 1)
                print "file\t" path "\t" digest
            }
        ' >> "$rows"
    fi

    LC_ALL=C sort -u "$rows" > "$out"
    rm -f "$list" "$rows" "$regulars"
}

capture_vendor_files_manifest() {
    local r="$1"
    local manifest="$r/$VENDOR_MANIFEST_REL"
    local tmp="$TARGET/.vendor-manifest-$$"
    write_vendor_files_manifest "$r" "$tmp"
    mv "$tmp" "$manifest"
}

vendor_package_files_pristine() {
    local r="$1"
    local expected="$r/$VENDOR_MANIFEST_REL"
    local actual="$TARGET/.vendor-check-$$"
    [ -s "$expected" ] || return 1
    write_vendor_files_manifest "$r" "$actual"
    if cmp -s "$expected" "$actual"; then
        rm -f "$actual"
        return 0
    fi
    echo "error: package-owned files differ from the imported Arch packages:" >&2
    diff -u "$expected" "$actual" | sed -n '1,200p' >&2 || true
    rm -f "$actual"
    return 1
}

pacman_offline_repo_ready() {
    local r="$1"
    local entry rel sha path actual target repo_db sync_db
    for entry in "${ARCH_OFFLINE_REPO_ARTIFACTS[@]}" "${ARCH_OFFLINE_REPO_SIGNATURES[@]}"; do
        rel="${entry%:*}"
        sha="${entry##*:}"
        path="$r/$ARCH_OFFLINE_REPO_REL/$rel"
        [ -s "$path" ] || return 1
        actual="$(sha256_of "$path")"
        [ "$actual" = "$sha" ] || return 1
    done
    for entry in "${ARCH_OFFLINE_REPO_PACKAGES[@]}"; do
        rel="${entry%:*}"
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

graphics_stage_ready() {
    if ! graphics_enabled; then
        return 0
    fi
    [ -x "$1/usr/bin/sudo" ] \
        && [ -x "$1/usr/bin/Xorg" ] \
        && [ -x "$1/usr/bin/startx" ] \
        && [ -x "$1/usr/bin/twm" ] \
        && [ -x "$1/usr/bin/xterm" ] \
        && [ -x "$1/usr/bin/lightdm" ] \
        && [ -x "$1/usr/bin/lightdm-gtk-greeter" ] \
        && [ -x "$1/usr/bin/startxfce4" ] \
        && [ -x "$1/usr/bin/xfwm4" ] \
        && [ -x "$1/usr/bin/xfce4-panel" ] \
        && [ -x "$1/usr/bin/xfce4-appfinder" ] \
        && [ -x "$1/usr/bin/xfdesktop" ] \
        && [ -x "$1/usr/bin/xfsettingsd" ] \
        && [ -x "$1/usr/bin/xfce4-settings-manager" ] \
        && [ -x "$1/usr/bin/xfce4-terminal" ] \
        && [ -x "$1/usr/bin/nano" ] \
        && [ -x "$1/usr/bin/firefox" ] \
        && [ -x "$1/usr/bin/dbus-launch" ] \
        && [ -e "$1/usr/lib/xorg/modules/drivers/fbdev_drv.so" ] \
        && [ -e "$1/usr/lib/xorg/modules/input/libinput_drv.so" ] \
        && [ -e "$1/usr/lib/xorg/modules/input/evdev_drv.so" ] \
        && [ -d "$1/var/lib/pacman/local/sudo-1.9.17.p2-2" ] \
        && [ -d "$1/var/lib/pacman/local/xorg-server-21.1.22-2" ] \
        && [ -d "$1/var/lib/pacman/local/xf86-video-fbdev-0.5.1-1" ] \
        && [ -d "$1/var/lib/pacman/local/xf86-input-evdev-2.11.0-1" ] \
        && [ -d "$1/var/lib/pacman/local/xorg-xinit-1.4.4-1" ] \
        && [ -d "$1/var/lib/pacman/local/xorg-twm-1.0.13.1-1" ] \
        && [ -d "$1/var/lib/pacman/local/xterm-410-1" ] \
        && [ -d "$1/var/lib/pacman/local/nano-9.0-1" ] \
        && [ -d "$1/var/lib/pacman/local/firefox-151.0.2-1" ] \
        && [ -e "$1/usr/share/fonts/misc/6x13-ISO8859-1.pcf.gz" ] \
        && [ -d "$1/var/lib/pacman/local/xorg-fonts-misc-1.0.4-2" ] \
        && find "$1/var/lib/pacman/local" -maxdepth 1 -type d -name 'noto-fonts-cjk-*' -print -quit \
            | grep -q .
}

graphics_pacman_database_ready() {
    if ! graphics_enabled; then
        return 0
    fi
    local r="$1"
    "$r/usr/lib/ld-linux-x86-64.so.2" \
        --library-path "$r/usr/lib" \
        "$r/usr/bin/pacman" \
        --database \
        --check \
        --config "$r/$ARCH_PACMAN_CONFIG" \
        --root "$r" \
        --dbpath "$r/var/lib/pacman" \
        --disable-sandbox \
        >/dev/null 2>&1 \
        && "$r/usr/lib/ld-linux-x86-64.so.2" \
            --library-path "$r/usr/lib" \
            "$r/usr/bin/pacman" \
            --query \
            --check \
            --config "$r/$ARCH_PACMAN_CONFIG" \
            --root "$r" \
            --dbpath "$r/var/lib/pacman" \
            --disable-sandbox \
            nano firefox noto-fonts-cjk \
            >/dev/null 2>&1
}

graphics_runtime_cache_ready() {
    if ! graphics_enabled; then
        return 0
    fi
    [ -x "$1/usr/bin/glycin-thumbnailer" ] \
        && [ -s "$1/usr/lib/libgdk_pixbuf-2.0.so.0.4400.6" ] \
        && [ "$(readlink "$1/usr/lib/libgdk_pixbuf-2.0.so.0" 2>/dev/null || true)" = "libgdk_pixbuf-2.0.so.0.4400.6" ] \
        && [ -x "$1/usr/lib/glycin-loaders/2+/glycin-image-rs" ] \
        && [ -x "$1/usr/lib/glycin-loaders/2+/glycin-svg" ] \
        && [ -s "$1/usr/share/glib-2.0/schemas/gschemas.compiled" ] \
        && [ -s "$1/usr/share/icons/hicolor/icon-theme.cache" ] \
        && [ -s "$1/usr/share/icons/AdwaitaLegacy/icon-theme.cache" ] \
        && [ -s "$1/usr/share/fonts/misc/fonts.dir" ] \
        && [ -L "$1/etc/fonts/conf.d/45-generic.conf" ] \
        && [ -L "$1/etc/fonts/conf.d/60-generic.conf" ] \
        && find "$1/var/cache/fontconfig" -type f -size +0c -print -quit 2>/dev/null \
            | grep -q . \
        && [ -s "$1/usr/share/mime/mime.cache" ] \
        && { [ ! -d "$1/usr/lib/gio/modules" ] \
            || [ -s "$1/usr/lib/gio/modules/giomodule.cache" ]; }
}

stage_ready() {
    [ -e "$STAGE_STAMP" ] \
        && [ -e "$STAGE/$(basename "$VENDOR_POLICY_STAMP")" ] \
        && [ -e "$STAGE/.lupos-profile" ] \
        && [ -e "$STAGE/usr/lib/systemd/systemd" ] \
        && [ -e "$STAGE/usr/bin/bash" ] \
        && [ -e "$STAGE/usr/bin/pacman" ] \
        && [ -e "$STAGE/etc/systemd/network/10-lupos-qemu.network" ] \
        && grep -q '^ConfigureWithoutCarrier=yes$' "$STAGE/etc/systemd/network/10-lupos-qemu.network" \
        && grep -q '^DNSDefaultRoute=yes$' "$STAGE/etc/systemd/network/10-lupos-qemu.network" \
        && [ -e "$STAGE/usr/lib/systemd/system/lupos-qemu-link-up.service" ] \
        && grep -q '^ExecStart=/usr/bin/ip link set dev eth0 up$' "$STAGE/usr/lib/systemd/system/lupos-qemu-link-up.service" \
        && [ "$(readlink "$STAGE/etc/systemd/system/multi-user.target.wants/lupos-qemu-link-up.service" 2>/dev/null || true)" = "/usr/lib/systemd/system/lupos-qemu-link-up.service" ] \
        && pacman_config_ready "$STAGE" \
        && vendor_package_configs_pristine "$STAGE" \
        && vendor_package_files_pristine "$STAGE" \
        && pacman_offline_repo_ready "$STAGE" \
        && pam_systemd_ready "$STAGE" \
        && graphics_stage_ready "$STAGE" \
        && graphics_pacman_database_ready "$STAGE" \
        && graphics_runtime_cache_ready "$STAGE"
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
    local cached_repo="$CACHE/arch-repo/$ARCH_REPO_SNAPSHOT/$rel"
    local cached_pkg="$CACHE/pacman-graphics/$(basename "$rel")"
    if [ -f "$cached_repo" ]; then
        local repo_actual
        repo_actual="$(sha256_of "$cached_repo")"
        [ "$repo_actual" = "$expected" ] || die "SHA-256 mismatch for cached $cached_repo: expected $expected, got $repo_actual"
        mkdir -p "$(dirname "$dst")"
        cp "$cached_repo" "$dst"
        return
    fi
    if [[ "$rel" == *.pkg.tar.zst ]] && [ -f "$cached_pkg" ]; then
        local cached_actual
        cached_actual="$(sha256_of "$cached_pkg")"
        [ "$cached_actual" = "$expected" ] || die "SHA-256 mismatch for cached $cached_pkg: expected $expected, got $cached_actual"
        mkdir -p "$(dirname "$dst")"
        cp "$cached_pkg" "$dst"
        return
    fi
    download_arch_repo_file "$rel" "$expected" "$dst"
}

stage_arch_pacman_package_cache() {
    local cache_dir="$ARCH_ROOTFS/var/cache/pacman/pkg"
    mkdir -p "$cache_dir"
    local entry rel base
    for entry in "${ARCH_OFFLINE_REPO_PACKAGES[@]}"; do
        rel="${entry%:*}"
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
    # Import the pinned Arch repository databases byte-for-byte. They are
    # vendor artifacts: never filter their metadata or rebuild their tarballs.
    download_arch_offline_repo_artifact \
        "core/os/x86_64/core.db" \
        "45037c1a6abb70a08cd225f1f2e98f6f1a0140117eba54a24843b581bf884a56"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/extra.db" \
        "2c4b923190d67f414ee981a020ca00a9f46c0e4ac44efa33fc067e2369e0387d"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/fastfetch-2.63.1-1-x86_64.pkg.tar.zst" \
        "d36690ae3a1c342da660ac499655eba532b0ceab58c976b92d943892c1ed232e"
    download_arch_offline_repo_artifact \
        "core/os/x86_64/gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst" \
        "95b97f61aacc075e85465a7d5e1c99d1b249b4eba63081a170482cdc8791f799"
    download_arch_offline_repo_artifact \
        "core/os/x86_64/nano-9.0-1-x86_64.pkg.tar.zst" \
        "2b52fdaafc70e63511f3b85d98c34f26151b0498fa6f0e61fcb4be4b0d754edf"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst" \
        "96518629c05db726744469eef47498bc15992e0ed499a0123c9f8917ff404cd6"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/vim-9.2.0573-1-x86_64.pkg.tar.zst" \
        "f375bc1779e4b595d0e3cdd7ba3a20eebaed9c7f16cb3e751589a27fdda174b1"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst" \
        "64dcc7ccaa5460b93ce1c76a9e104bcbb373d3d15fb5abc01a1192f4607e9d2e"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/freetype2-2.14.3-1-x86_64.pkg.tar.zst" \
        "fcaa410420dea42779d02aa76f1cc95d8430bdc52071ac6219d33306899b8655"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libice-1.1.2-1-x86_64.pkg.tar.zst" \
        "bb613be39e5bc1707a39f895c178674bbda52f022da51612e7a10a386608e107"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libpng-1.6.58-1-x86_64.pkg.tar.zst" \
        "8d80045f15f6339b2284db5df06a4287225cc2389c24dbb2fbc23458f6887ee5"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libsm-1.2.6-1-x86_64.pkg.tar.zst" \
        "ce334b07a9701ba6ef4d610257500f7b035dc1cdc0a658e056d293547cde0976"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libutempter-1.2.3-1-x86_64.pkg.tar.zst" \
        "e1dd15aed4cd76b42729d5dc58d4db3dd49d6c5fe08b33abee0df3bcfc98fd13"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libx11-1.8.13-1-x86_64.pkg.tar.zst" \
        "251f58e0a9bc2cd69b8e708f239914e48e3a91cd1822220806d273be873d026f"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxau-1.0.12-1-x86_64.pkg.tar.zst" \
        "605c8b059c36792f4e0cc235acadf39d0762df6c7878825a1be01a00ae7ea21e"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxaw-1.0.16-2-x86_64.pkg.tar.zst" \
        "79966ba7df3cf46bbcac4292384247b0070bfd036f3a55dbcd9121cb4ccf8a38"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxcb-1.17.0-1-x86_64.pkg.tar.zst" \
        "2b2e7ac64b1d56c08a227c10bcab179605f2773f31db0a8c89f49f4e5b2f1292"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxdmcp-1.1.5-2-x86_64.pkg.tar.zst" \
        "623c957c2fd4b427a0f5a531da44931f9f66521391ee0bd0e635479947036b65"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxext-1.3.7-1-x86_64.pkg.tar.zst" \
        "ac56905dc51bb652eca5f706fd7e7bb7ea81d4e057a236139fc769ce5ea10cf1"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxft-2.3.9-1-x86_64.pkg.tar.zst" \
        "a7841ed8e67dc3f94eca4672c8961329dfdd6b67836f8ac720a246d3aab02ecb"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxmu-1.3.1-1-x86_64.pkg.tar.zst" \
        "31870eb5ad1911880cd7b2f3a59292b644d84b873ed62a1a7c36b9ac26bc08a5"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxpm-3.5.19-1-x86_64.pkg.tar.zst" \
        "cb4df58d300485410132dcd0deeb209c0ef9a6e8d5280ec0f4cb532aba2c208e"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxrender-0.9.12-1-x86_64.pkg.tar.zst" \
        "fed0389073d5b107074eaab48cefcc2716e607865142cde5b579c8ceeefea142"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/libxt-1.3.1-1-x86_64.pkg.tar.zst" \
        "5d4ee1f73c946cdd9b908b127157d6d826d198be982c91e199daf6bdd7f6b9ec"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xcb-proto-1.17.0-4-any.pkg.tar.zst" \
        "98f661ef7c7e05eb7a687e859cf123a92806ceed8f55c0d70ddbac799988239a"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst" \
        "df9e0d1f29ae0135a49b17157841a4cc7c97869f9f27fd7480fff64f9afb94b7"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst" \
        "edf4f98e4c787074b3d182daf9eadfacee3ce14785397ccd63fe11f9ffb1c903"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst" \
        "b80ea53e86c02b3c7697c8353ccc41df80278b57f0134f6a608be126a0724bb3"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst" \
        "e916cb35a6a3031ddd6c6d49d795bcd702dcc532debb911dc97cec715a235c30"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xorgproto-2025.1-1-any.pkg.tar.zst" \
        "f7bf3eed570618511fb53cc1bd32c2f1ee82e02662075436a59e6e50436f30de"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/xterm-410-1-x86_64.pkg.tar.zst" \
        "85b31fbadc47676215007d2fb08115a237e30c385e382d98f7cf6c59a77d9fa9"
    download_arch_offline_repo_artifact \
        "extra/os/x86_64/yyjson-0.12.0-1-x86_64.pkg.tar.zst" \
        "a25b2c4be6039c36ef9a7f440ac984772e5052aacac24f6046757053c7d77b58"
    local signature rel sha
    for signature in "${ARCH_OFFLINE_REPO_SIGNATURES[@]}"; do
        rel="${signature%:*}"
        sha="${signature##*:}"
        download_arch_offline_repo_artifact "$rel" "$sha"
    done
    stage_arch_pacman_sync_dbs
    stage_arch_pacman_package_cache
    stage_arch_pacman_package_aliases
}

stage_arch_pacman_keyring() {
    local source_dir="$ARCH_ROOTFS/usr/share/pacman/keyrings"
    local source_keyring="$source_dir/archlinux.gpg"
    local source_trust="$source_dir/archlinux-trusted"
    local gpgdir="$ARCH_ROOTFS/$ARCH_PACMAN_GPG_DIR"
    [ -s "$source_keyring" ] || die "missing Arch package keyring: $source_keyring"
    [ -s "$source_trust" ] || die "missing Arch package ownertrust: $source_trust"

    # The bootstrap archive deliberately ships the vendor keyring under
    # /usr/share but leaves pacman's local GPG database uninitialized. Build
    # that local state without editing the vendor keyring or pacman policy.
    # Trust the five snapshot-pinned Arch master keys as roots; their
    # signatures establish validity for the individual package-signing keys.
    require_command gpg
    local work="$TARGET/.arch-keyring-$$"
    safe_clean_dir "$work"
    mkdir -m 700 -p "$work"
    gpg --batch --quiet --no-permission-warning --homedir "$work" \
        --import "$source_keyring" \
        || die "failed to import the Arch package keyring"
    gpg --batch --no-permission-warning --homedir "$work" \
        --with-colons --fingerprint \
        | awk -F: '$1 == "fpr" { print $10 ":1:" }' > "$work/all-ownertrust"
    gpg --batch --quiet --no-permission-warning --homedir "$work" \
        --import-ownertrust "$work/all-ownertrust" \
        || die "failed to initialize Arch package-key ownertrust"
    awk -F: 'NF >= 2 { print $1 ":6:" }' "$source_trust" > "$work/archlinux-ultimate"
    gpg --batch --quiet --no-permission-warning --homedir "$work" \
        --import-ownertrust "$work/archlinux-ultimate" \
        || die "failed to establish trust in the Arch master keys"
    gpg --batch --quiet --no-permission-warning --homedir "$work" \
        --update-trustdb </dev/null \
        || die "failed to build the Arch package trust database"

    # libalpm checks for this legacy marker while modern GnuPG reads the kbx.
    : > "$work/pubring.gpg"
    chmod 644 "$work/pubring.gpg" "$work/pubring.kbx" "$work/trustdb.gpg"
    safe_clean_dir "$gpgdir"
    mkdir -p "$gpgdir"
    cp "$work/pubring.gpg" "$work/pubring.kbx" "$work/trustdb.gpg" "$gpgdir/"
    safe_clean_dir "$work"
}

stage_arch_pacman_sync_dbs() {
    mkdir -p "$ARCH_ROOTFS/var/lib/pacman/sync"
    cp "$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/core/os/x86_64/core.db" \
        "$ARCH_ROOTFS/var/lib/pacman/sync/core.db"
    cp "$ARCH_ROOTFS/$ARCH_OFFLINE_REPO_REL/extra/os/x86_64/extra.db" \
        "$ARCH_ROOTFS/var/lib/pacman/sync/extra.db"
}

find_fakeroot_lib_dir() {
    local dir
    for dir in \
        /usr/lib/x86_64-linux-gnu/libfakeroot \
        /usr/lib64/libfakeroot \
        /usr/lib/libfakeroot
    do
        [ -e "$dir/libfakeroot-sysv.so" ] || continue
        printf '%s\n' "$dir"
        return 0
    done
    return 1
}

write_arch_archive_pacman_conf() {
    local conf="$1"
    local hook_dir="$2"
    write_file "$conf" <<EOF
[options]
Architecture = x86_64
SigLevel = Never
DisableSandbox
HookDir = $hook_dir

[core]
SigLevel = Never
Server = $ARCH_REPO_BASE_URL/core/os/x86_64

[extra]
SigLevel = Never
Server = $ARCH_REPO_BASE_URL/extra/os/x86_64
EOF
}

install_arch_graphics_packages() {
    if ! graphics_enabled; then
        return 0
    fi

    local work="$TARGET/.graphics-pacman-$$"
    safe_clean_dir "$work"
    mkdir -p "$work/hooks" "$work/gpg" "$CACHE/pacman-graphics"
    write_arch_archive_pacman_conf "$work/pacman.conf" "$work/hooks"

    if graphics_stage_ready "$ARCH_ROOTFS"; then
        log "Graphics packages already present in $ARCH_ROOTFS"
    else
        require_command fakeroot
        local fakeroot_lib_dir
        fakeroot_lib_dir="$(find_fakeroot_lib_dir)" || die "fakeroot library directory not found"

        local full_core_db="$CACHE/arch-repo/$ARCH_REPO_SNAPSHOT/core/os/x86_64/core.db"
        local full_extra_db="$CACHE/arch-repo/$ARCH_REPO_SNAPSHOT/extra/os/x86_64/extra.db"
        [ -s "$full_core_db" ] || die "missing full Arch core db: $full_core_db"
        [ -s "$full_extra_db" ] || die "missing full Arch extra db: $full_extra_db"

        cp "$full_core_db" "$ARCH_ROOTFS/var/lib/pacman/sync/core.db"
        cp "$full_extra_db" "$ARCH_ROOTFS/var/lib/pacman/sync/extra.db"

        log "Installing X11 graphics packages into Arch rootfs"
        fakeroot -- \
            "$ARCH_ROOTFS/usr/lib/ld-linux-x86-64.so.2" \
            --library-path "$fakeroot_lib_dir:$ARCH_ROOTFS/usr/lib" \
            "$ARCH_ROOTFS/usr/bin/pacman" \
            -S \
            --config "$work/pacman.conf" \
            --root "$ARCH_ROOTFS" \
            --dbpath "$ARCH_ROOTFS/var/lib/pacman" \
            --cachedir "$CACHE/pacman-graphics" \
            --gpgdir "$work/gpg" \
            --hookdir "$work/hooks" \
            --noconfirm \
            --needed \
            --noscriptlet \
            --disable-sandbox \
            "${ARCH_GRAPHICS_PACKAGES[@]}"
    fi

    # Use pacman's own dependency checker against the completed local package
    # database.  This makes a broken Firefox dependency closure fail the image
    # build even if the top-level executable happened to be unpacked.
    log "Checking installed graphics package dependency closure with pacman"
    "$ARCH_ROOTFS/usr/lib/ld-linux-x86-64.so.2" \
        --library-path "$ARCH_ROOTFS/usr/lib" \
        "$ARCH_ROOTFS/usr/bin/pacman" \
        --database \
        --check \
        --config "$work/pacman.conf" \
        --root "$ARCH_ROOTFS" \
        --dbpath "$ARCH_ROOTFS/var/lib/pacman" \
        --disable-sandbox
    "$ARCH_ROOTFS/usr/lib/ld-linux-x86-64.so.2" \
        --library-path "$ARCH_ROOTFS/usr/lib" \
        "$ARCH_ROOTFS/usr/bin/pacman" \
        --query \
        --check \
        --config "$work/pacman.conf" \
        --root "$ARCH_ROOTFS" \
        --dbpath "$ARCH_ROOTFS/var/lib/pacman" \
        --disable-sandbox \
        nano firefox noto-fonts-cjk

    stage_arch_pacman_sync_dbs
    safe_clean_dir "$work"
}

# Rebuild the core-X `fonts.dir` index for every bitmap font directory under the
# given root.  pacman runs with `--noscriptlet`, so the packaged
# mkfontdir/mkfontscale hook never fires and the fonts.alias/*.pcf.gz files ship
# without the index the X server and xterm need to resolve core fonts. Generate
# it here with the host mkfontdir.
# Called on the final stage (not gated by the graphics-install early-return), so
# it always runs when the graphics profile is enabled.
generate_arch_font_indexes() {
    if ! graphics_enabled; then
        return 0
    fi
    local root="$1"
    local base="$root/usr/share/fonts"
    [ -d "$base" ] || return 0
    require_command mkfontdir
    local dir f has_bitmap
    while IFS= read -r -d '' dir; do
        # Non-matching globs stay literal, so probe each candidate with -e
        # instead of letting `ls` fail the whole directory.
        has_bitmap=0
        for f in "$dir"/*.pcf.gz "$dir"/*.pcf "$dir"/*.bdf; do
            if [ -e "$f" ]; then
                has_bitmap=1
                break
            fi
        done
        if [ "$has_bitmap" = 1 ]; then
            log "Generating fonts.dir in ${dir#"$root"/}"
            ( cd "$dir" && mkfontdir . )
        fi
    done < <(find "$base" -type d -print0)
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

write_lupos_pacman_config() {
    write_file "$ARCH_ROOTFS/$ARCH_PACMAN_CONFIG" <<EOF
[options]
Architecture = auto
CheckSpace
SigLevel = Required DatabaseOptional
LocalFileSigLevel = Optional
DisableSandbox
$ARCH_PACMAN_XFER_COMMAND

[core]
$ARCH_PACMAN_SERVER
$ARCH_PACMAN_ARCHIVE_SERVER

[extra]
$ARCH_PACMAN_SERVER
$ARCH_PACMAN_ARCHIVE_SERVER
EOF
}

apply_lupos_overlay() {
    local S="$ARCH_ROOTFS"
    log "Applying Lupos overlay"

    write_file "$S/etc/hostname" <<< "lupos"

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
    http://*|https://*)
        exec /usr/bin/curl --disable --fail --location --proto '=http,https' --proto-redir '=http,https' --continue-at - --output "$dst" "$src"
        ;;
    *)
        echo "unsupported pacman transfer URL: $src" >&2
        exit 2
        ;;
esac

base=${src##*/}
if [ ! -e "$src" ]; then
    case "$src" in
        *.sig) exit 1 ;;
    esac
    echo "missing pacman transfer source: $src" >&2
    exit 1
fi

alias=
case "$base" in
    fastfetch-2.63.1-1-x86_64.pkg.tar.zst) alias=/p/fastfetch ;;
    fontconfig-2:2.17.1-1-x86_64.pkg.tar.zst) alias=/p/fontconfig ;;
    freetype2-2.14.3-1-x86_64.pkg.tar.zst) alias=/p/freetype2 ;;
    gpm-1.20.7.r38.ge82d1a6-6-x86_64.pkg.tar.zst) alias=/p/g ;;
    libice-1.1.2-1-x86_64.pkg.tar.zst) alias=/p/libice ;;
    libpng-1.6.58-1-x86_64.pkg.tar.zst) alias=/p/libpng ;;
    libsm-1.2.6-1-x86_64.pkg.tar.zst) alias=/p/libsm ;;
    libutempter-1.2.3-1-x86_64.pkg.tar.zst) alias=/p/libutempter ;;
    libx11-1.8.13-1-x86_64.pkg.tar.zst) alias=/p/libx11 ;;
    libxau-1.0.12-1-x86_64.pkg.tar.zst) alias=/p/libxau ;;
    libxaw-1.0.16-2-x86_64.pkg.tar.zst) alias=/p/libxaw ;;
    libxcb-1.17.0-1-x86_64.pkg.tar.zst) alias=/p/libxcb ;;
    libxdmcp-1.1.5-2-x86_64.pkg.tar.zst) alias=/p/libxdmcp ;;
    libxext-1.3.7-1-x86_64.pkg.tar.zst) alias=/p/libxext ;;
    libxft-2.3.9-1-x86_64.pkg.tar.zst) alias=/p/libxft ;;
    libxmu-1.3.1-1-x86_64.pkg.tar.zst) alias=/p/libxmu ;;
    libxpm-3.5.19-1-x86_64.pkg.tar.zst) alias=/p/libxpm ;;
    libxrender-0.9.12-1-x86_64.pkg.tar.zst) alias=/p/libxrender ;;
    libxt-1.3.1-1-x86_64.pkg.tar.zst) alias=/p/libxt ;;
    nano-9.0-1-x86_64.pkg.tar.zst) alias=/p/n ;;
    vim-9.2.0573-1-x86_64.pkg.tar.zst) alias=/p/v ;;
    vim-runtime-9.2.0573-1-x86_64.pkg.tar.zst) alias=/p/r ;;
    xcb-proto-1.17.0-4-any.pkg.tar.zst) alias=/p/xcb-proto ;;
    xorg-xauth-1.1.5-1-x86_64.pkg.tar.zst) alias=/p/xorg-xauth ;;
    xorg-xinit-1.4.4-1-x86_64.pkg.tar.zst) alias=/p/xorg-xinit ;;
    xorg-xmodmap-1.0.11-2-x86_64.pkg.tar.zst) alias=/p/xorg-xmodmap ;;
    xorg-xrdb-1.2.2-2-x86_64.pkg.tar.zst) alias=/p/xorg-xrdb ;;
    xorgproto-2025.1-1-any.pkg.tar.zst) alias=/p/xorgproto ;;
    xterm-410-1-x86_64.pkg.tar.zst) alias=/p/xterm ;;
    yyjson-0.12.0-1-x86_64.pkg.tar.zst) alias=/p/yyjson ;;
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
    install_arch_graphics_packages
    # Package contents are immutable input. Capture every path recorded by
    # pacman's local database before adding any Lupos-owned files.
    capture_vendor_files_manifest "$S"

    write_file "$S/.lupos-profile" <<'EOF'
distro=arch
builder=lupos-unprivileged-bootstrap
EOF

    mkdir -p "$S/etc/systemd/network"
    write_file "$S/etc/systemd/network/10-lupos-qemu.network" <<'EOF'
[Match]
Name=e* eth* en*

[Network]
ConfigureWithoutCarrier=yes
Address=10.0.2.15/24
Gateway=10.0.2.2
DNS=10.0.2.3
DNSDefaultRoute=yes
EOF

    mkdir -p "$S/usr/lib/systemd/system"
    write_file "$S/usr/lib/systemd/system/lupos-qemu-link-up.service" <<'EOF'
[Unit]
Description=Activate the Lupos QEMU network interface
After=systemd-modules-load.service
Before=systemd-networkd.service
Wants=systemd-modules-load.service

[Service]
Type=oneshot
ExecStart=/usr/bin/ip link set dev eth0 up
RemainAfterExit=yes
EOF

    mkdir -p \
        "$S/etc/systemd/system/getty.target.wants" \
        "$S/etc/systemd/system/getty@tty1.service.d" \
        "$S/etc/systemd/system/multi-user.target.wants"

    ln -sfn /usr/lib/systemd/system/getty@.service \
        "$S/etc/systemd/system/getty.target.wants/getty@tty1.service"
    ln -sfn /usr/lib/systemd/system/lupos-qemu-link-up.service \
        "$S/etc/systemd/system/multi-user.target.wants/lupos-qemu-link-up.service"

    write_file "$S/etc/systemd/system/getty@tty1.service.d/lupos.conf" <<'EOF'
[Service]
Type=simple
Restart=always
RestartSec=0
ExecStart=
ExecStart=-/sbin/agetty --noclear --nohostname tty1 linux
EOF

    for svc in systemd-networkd systemd-resolved; do
        [ -f "$S/usr/lib/systemd/system/${svc}.service" ] || continue
        ln -sfn "/usr/lib/systemd/system/${svc}.service" \
            "$S/etc/systemd/system/multi-user.target.wants/${svc}.service"
    done

    stage_arch_pacman_keyring
    write_lupos_pacman_config
    : > "$VENDOR_POLICY_STAMP"
}

copy_to_stage() {
    log "Staging into $STAGE"
    mkdir -p "$(dirname "$STAGE")"
    safe_clean_dir "$STAGE"
    mkdir -p "$STAGE"
    cp -a "$ARCH_ROOTFS/." "$STAGE/"
    generate_arch_fontconfig "$STAGE"
    generate_arch_font_indexes "$STAGE"
    generate_arch_gtk_caches "$STAGE"
}

# pacman runs with `--noscriptlet`, so fontconfig's packaged post_install does
# not populate /etc/fonts/conf.d from the vendor conf.default selection and
# does not build the system font cache.  Reproduce those two vendor operations
# against the staged root.  Merely shipping the TTF files is insufficient:
# modern applications resolve generic families through these configuration
# links, and Firefox aborts during graphics initialization if no usable default
# font can be selected.
generate_arch_fontconfig() {
    if ! graphics_enabled; then
        return 0
    fi
    local root="$1"
    local defaults="$root/usr/share/fontconfig/conf.default"
    local conf_d="$root/etc/fonts/conf.d"
    [ -d "$defaults" ] || return 0

    mkdir -p "$conf_d"
    local default name
    while IFS= read -r -d '' default; do
        name="${default##*/}"
        ln -sfn "/usr/share/fontconfig/conf.default/$name" "$conf_d/$name"
    done < <(find "$defaults" -mindepth 1 -maxdepth 1 -type l -print0)

    local ld="$root/usr/lib/ld-linux-x86-64.so.2"
    local fc_cache="$root/usr/bin/fc-cache"
    if [ -x "$ld" ] && [ -x "$fc_cache" ]; then
        log "Rebuilding staged fontconfig cache"
        "$ld" --library-path "$root/usr/lib" "$fc_cache" \
            --really-force --system-only --sysroot "$root" \
            || die "failed to rebuild staged fontconfig cache"
    fi
}

# pacman runs with `--noscriptlet`, so the GTK/GLib post-install hooks never
# fire.  XFCE (GTK3) refuses to start, or renders unthemed/iconless, without
# the compiled GSettings schemas, the gdk-pixbuf loader cache (needed to load
# PNG/SVG icons), and the per-theme icon caches.  Generate them host-side with
# the tools shipped inside the staged rootfs — mirrors what
# `generate_arch_font_indexes` does for the core-X fonts.dir.
generate_arch_gtk_caches() {
    if ! graphics_enabled; then
        return 0
    fi
    local root="$1"
    local ld="$root/usr/lib/ld-linux-x86-64.so.2"
    [ -x "$ld" ] || return 0
    # Run a staged ELF binary against the staged libraries (the host may not
    # have the same glib), like install_arch_graphics_packages does for pacman.
    run_staged() {
        local bin="$root/$1"
        shift
        [ -x "$bin" ] || return 0
        "$ld" --library-path "$root/usr/lib" "$bin" "$@" 2>/dev/null
    }

    # shared-mime-info's package hook is disabled with every other pacman
    # scriptlet. GdkPixbuf consults mime.cache before selecting even its built-in
    # PNG loader, so without this database valid PNG icons report "unknown type".
    if [ -x "$root/usr/bin/update-mime-database" ] \
        && [ -d "$root/usr/share/mime/packages" ]; then
        log "Updating shared MIME database"
        run_staged usr/bin/update-mime-database "$root/usr/share/mime" \
            || die "failed to update staged MIME database"
    fi

    if [ -d "$root/usr/share/glib-2.0/schemas" ]; then
        log "Compiling GSettings schemas"
        run_staged usr/bin/glib-compile-schemas "$root/usr/share/glib-2.0/schemas" \
            || die "failed to compile staged GSettings schemas"
    fi

    # gdk-pixbuf loader cache — written to the versioned loaders dir the
    # library looks up at runtime.
    local loaders_conf
    loaders_conf="$(find "$root/usr/lib/gdk-pixbuf-2.0" -name loaders.cache 2>/dev/null | head -1 || true)"
    if [ -x "$root/usr/bin/gdk-pixbuf-query-loaders" ]; then
        local loaders_dir
        loaders_dir="$(find "$root/usr/lib/gdk-pixbuf-2.0" -type d -name loaders 2>/dev/null | head -1 || true)"
        if [ -n "$loaders_dir" ]; then
            log "Building gdk-pixbuf loaders cache"
            GDK_PIXBUF_MODULEDIR="$loaders_dir" \
                "$ld" --library-path "$root/usr/lib" \
                "$root/usr/bin/gdk-pixbuf-query-loaders" \
                > "${loaders_conf:-$(dirname "$loaders_dir")/loaders.cache}" 2>/dev/null \
                || die "failed to build staged gdk-pixbuf loader cache"
        fi
    fi

    # GIO extension modules (the dconf/xfconf GSettings backends, GnuTLS…)
    # are normally indexed by each package's gio-querymodules hook, which
    # --noscriptlet suppressed. Without giomodule.cache GIO must dlopen every
    # module to discover extension points; regenerate the cache host-side.
    if [ -x "$root/usr/bin/gio-querymodules" ] \
        && [ -d "$root/usr/lib/gio/modules" ]; then
        log "Building GIO module cache"
        run_staged usr/bin/gio-querymodules "$root/usr/lib/gio/modules" \
            || die "failed to build staged GIO module cache"
    fi

    # Per-theme icon caches so GTK finds icons quickly and correctly.
    if [ -x "$root/usr/bin/gtk-update-icon-cache" ]; then
        local theme
        for theme in "$root"/usr/share/icons/*/; do
            [ -f "$theme/index.theme" ] || continue
            log "Updating icon cache for ${theme#"$root"/}"
            run_staged usr/bin/gtk-update-icon-cache -q -f "$theme" \
                || die "failed to update staged icon cache for ${theme#"$root"/}"
        done
    fi
}

validate_stage() {
    local bad=0
    for p in \
        usr/lib/systemd/systemd \
        usr/bin/bash \
        usr/bin/pacman \
        etc/os-release \
        etc/pacman.conf \
        "$ARCH_PACMAN_CONFIG" \
        "$ARCH_PACMAN_GPG_DIR/pubring.gpg" \
        "$ARCH_PACMAN_GPG_DIR/pubring.kbx" \
        "$ARCH_PACMAN_GPG_DIR/trustdb.gpg" \
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
    pacman_config_ready "$STAGE" || die "staged pacman integration must retain Arch signature policy and initialize its keyring while defining the Lupos offline transport"
    vendor_package_configs_pristine "$STAGE" || die "package-owned pacman.conf or mirrorlist was modified"
    vendor_package_files_pristine "$STAGE" || die "a package-owned Arch file was modified after import"
    pacman_offline_repo_ready "$STAGE" || die "staged offline pacman repo is missing pinned database/package files or preseeded sync databases"
    pam_systemd_ready "$STAGE" || die "staged PAM systemd session hook is missing"
    graphics_stage_ready "$STAGE" || die "staged graphics profile is missing X11 packages"
    graphics_pacman_database_ready "$STAGE" || die "staged graphics profile has an inconsistent pacman dependency database or missing Firefox/nano files"
    graphics_runtime_cache_ready "$STAGE" || die "staged graphics profile is missing generated runtime caches"
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
