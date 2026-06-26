#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ARTIFACT_ROOT="$ROOT/target/xtask/abi-parity-suite"
JOBS="${JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)}"

if [[ -d "$HOME/.local/bin" ]]; then
    case ":$PATH:" in
        *":$HOME/.local/bin:"*) ;;
        *) export PATH="$HOME/.local/bin:$PATH" ;;
    esac
fi

usage() {
    echo "usage: $0 --suite <suite>|all" >&2
    exit 2
}

suite=""
while (($#)); do
    case "$1" in
        --suite)
            shift
            (($#)) || usage
            suite="$1"
            ;;
        *)
            usage
            ;;
    esac
    shift
done

[[ -n "$suite" ]] || usage

need_path() {
    [[ -e "$ROOT/$1" ]] || {
        echo "error: missing ABI parity input path: $1" >&2
        exit 1
    }
}

normalize_crlf_scripts() {
    local tree="$1"
    [[ -d "$tree" ]] || return 0
    find "$tree" -type f \( -name '*.sh' -o -name '*.bash' \) -print0 | while IFS= read -r -d '' file; do
        python3 - "$file" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
data = path.read_bytes()
normalized = data.replace(b"\r\n", b"\n")
if normalized != data:
    path.write_bytes(normalized)
PY
    done
}

normalize_crlf_matching() {
    local tree="$1"
    shift
    [[ -d "$tree" ]] || return 0
    ((
        $# > 0
    )) || return 0

    local expr=()
    while (($#)); do
        expr+=( -name "$1" -o )
        shift
    done
    unset 'expr[${#expr[@]}-1]'

    find "$tree" -type f \( "${expr[@]}" \) -print0 | while IFS= read -r -d '' file; do
        python3 - "$file" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
data = path.read_bytes()
normalized = data.replace(b"\r\n", b"\n")
if normalized != data:
    path.write_bytes(normalized)
PY
    done
}

manifest_entries() {
    local manifest="$1"
    python3 - "$manifest" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
for raw in manifest.read_text(encoding="utf-8").splitlines():
    line = raw.strip()
    if not line or line.startswith("#"):
        continue
    print(line)
PY
}

stage_suite_manifest() {
    local key="$1"
    local source_rel="$2"
    local dest_name="$3"
    local dest
    need_path "$source_rel"
    dest="$(suite_dir "$key")/build/$dest_name"
    mkdir -p "$(dirname "$dest")"
    cp "$ROOT/$source_rel" "$dest"
}

stage_sparse_image() {
    local path="$1"
    local size="$2"
    mkdir -p "$(dirname "$path")"
    rm -f "$path"
    truncate -s "$size" "$path"
}

ensure_kselftest_header_compat() {
    local include_root="$1"
    mkdir -p "$include_root/asm"
    python3 - "$include_root" <<'PY'
from pathlib import Path
import sys

include_root = Path(sys.argv[1])
asm_dir = include_root / "asm"
generic_dir = include_root / "asm-generic"
if not generic_dir.is_dir():
    raise SystemExit(0)

shim_prefix = "/* Auto-generated ABI parity kselftest compatibility shim. */\n"
for generic in generic_dir.glob("*.h"):
    target = asm_dir / generic.name
    if target.exists():
        continue
    target.write_text(
        f"{shim_prefix}#include <asm-generic/{generic.name}>\n",
        encoding="utf-8",
    )
PY
}

write_abi_parity_kselftest_runner() {
    local runner_path="$1"
cat > "$runner_path" <<'EOF'
#!/bin/bash
set -u

DEFAULT_ROOT="${ABI_PARITY_KSELFTEST_ROOT:-/opt/abi-parity-suite/kselftest}"
if [[ -f "$DEFAULT_ROOT/manifest.tsv" ]]; then
    ROOT="$DEFAULT_ROOT"
else
    script_path="${BASH_SOURCE[0]}"
    case "$script_path" in
        /*) ROOT="${script_path%/*}" ;;
        */*) ROOT="$(cd "${script_path%/*}" && pwd)" ;;
        *) ROOT="$(pwd)" ;;
    esac
fi
MANIFEST="$ROOT/manifest.tsv"
TEST_ROOT="$ROOT/tests"
TEST_TIMEOUT="${ABI_PARITY_TEST_TIMEOUT_SECS:-60s}"
TEST_KILL_AFTER="${ABI_PARITY_TEST_KILL_AFTER_SECS:-5s}"
TRANSHUGE_STRESS_DURATION="${ABI_PARITY_TRANSHUGE_STRESS_DURATION_SECS:-20}"
TEST_FILTER="${ABI_PARITY_KSELFTEST_FILTER:-}"
TEST_FILTER_PREFIX="${ABI_PARITY_KSELFTEST_FILTER_PREFIX:-}"
SKIP_META_WRAPPERS="${ABI_PARITY_KSELFTEST_SKIP_META_WRAPPERS:-0}"
if [[ -d "$ROOT/lib" ]]; then
    export LD_LIBRARY_PATH="$ROOT/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

parse_timeout_secs() {
    local value="$1"
    case "$value" in
        *s) printf '%s\n' "${value%s}" ;;
        *) printf '%s\n' "$value" ;;
    esac
}

run_with_deadline() {
    local test_path="$1"
    shift
    local timeout_secs kill_after_secs elapsed child rc
    timeout_secs="$(parse_timeout_secs "$TEST_TIMEOUT")"
    kill_after_secs="$(parse_timeout_secs "$TEST_KILL_AFTER")"
    "$test_path" "$@" &
    child=$!
    elapsed=0
    while kill -0 "$child" 2>/dev/null; do
        if (( elapsed >= timeout_secs )); then
            kill -TERM "$child" 2>/dev/null || true
            sleep "$kill_after_secs"
            kill -KILL "$child" 2>/dev/null || true
            wait "$child" 2>/dev/null || true
            return 124
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    wait "$child"
    rc=$?
    return "$rc"
}

prepare_test_environment() {
    local test_id="$1"
    case "$test_id" in
        mm:hugepage-mmap)
            echo 128 > /proc/sys/vm/nr_hugepages 2>/dev/null || true
            ;;
        mm:hugetlb-soft-offline)
            echo 8 > /proc/sys/vm/nr_hugepages 2>/dev/null || true
            ;;
    esac
}

[[ -f "$MANIFEST" ]] || {
    echo $'ABI_PARITY_RESULT\tkselftest\tfail\tmissing manifest.tsv'
    echo "ABI_PARITY_DONE"
    exit 1
}

ran_any=0
while IFS=$'\t' read -r test_id availability relpath notes; do
    [[ "$test_id" == "test_id" ]] && continue
    if [[ -n "$TEST_FILTER" && "$test_id" != "$TEST_FILTER" ]]; then
        continue
    fi
    if [[ -n "$TEST_FILTER_PREFIX" && "$test_id" != "$TEST_FILTER_PREFIX"* ]]; then
        continue
    fi
    if [[ "$SKIP_META_WRAPPERS" == "1" && "$test_id" == mm:ksft_*.sh ]]; then
        continue
    fi
    ran_any=1
    if [[ "$availability" != "ready" || "$relpath" == "-" ]]; then
        printf 'ABI_PARITY_RESULT\t%s\tskip\t%s\n' "$test_id" "${notes:-artifact unavailable}"
        continue
    fi

    test_path="$TEST_ROOT/$relpath"
    test_dir="${test_path%/*}"
    test_base="${test_path##*/}"
    test_args=()
    case "$test_id" in
        mm:transhuge-stress)
            test_args=(-d "$TRANSHUGE_STRESS_DURATION")
            ;;
    esac

    set +e
    prepare_test_environment "$test_id"
    (cd "$test_dir" && run_with_deadline "$test_path" "${test_args[@]}")
    rc=$?
    set -e
    status="fail"
    case "$rc" in
        0) status="pass" ;;
        4) status="skip" ;;
    esac

    printf 'ABI_PARITY_RESULT\t%s\t%s\texit=%s\n' \
        "$test_id" "$status" "$rc"
done < "$MANIFEST"

if [[ "$ran_any" -eq 0 ]]; then
    printf 'ABI_PARITY_RESULT\tkselftest:filter\tfail\tno manifest row matched %s\n' "${TEST_FILTER:-<empty>}"
fi

echo "ABI_PARITY_DONE"
EOF
    chmod +x "$runner_path"
}

kselftest_record_entry() {
    local manifest_tsv="$1"
    local test_id="$2"
    local availability="$3"
    local relpath="$4"
    local notes="$5"
    printf '%s\t%s\t%s\t%s\n' \
        "$test_id" "$availability" "$relpath" "$notes" >> "$manifest_tsv"
}

kselftest_install_ready_artifact() {
    local install_root="$1"
    local relpath="$2"
    local src="$3"
    local dst="$install_root/tests/$relpath"
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    chmod 755 "$dst"
}

kselftest_install_ready_source_artifact() {
    local install_root="$1"
    local relpath="$2"
    local src="$3"
    local dst="$install_root/tests/$relpath"
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    chmod 755 "$dst"
}

kselftest_stage_mm_support_files() {
    local install_root="$1"
    local mm_src="$ROOT/vendor/linux/tools/testing/selftests/mm"
    local dst="$install_root/tests/mm"
    mkdir -p "$dst"

    find "$mm_src" -maxdepth 1 -type f \
        \( -name '*.sh' -o -name 'settings' -o -name 'config' -o -name 'local_config.h' -o -name 'local_config.mk' \) \
        -exec cp {} "$dst/" \;
    find "$dst" -maxdepth 1 -type f -name '*.sh' -exec chmod 755 {} \;
}

kselftest_try_build_target() {
    local obj_root="$1"
    local cc_bin="$2"
    local build_log="$3"
    local dir="$4"
    local output_dir="$5"
    local target="$6"
    shift 6
    local khdr_includes="-isystem $obj_root/usr/include"
    local target_timeout="${KSELFTEST_TARGET_TIMEOUT_SECS:-20s}"
    local target_ldflags="${KSELFTEST_LDFLAGS:-}"

    mkdir -p "$output_dir"
    (
        cd "$ROOT/vendor/linux/tools/testing/selftests/$dir"
        timeout "$target_timeout" make \
            O="$obj_root" \
            OUTPUT="$output_dir" \
            CC="$cc_bin" \
            LDFLAGS="$target_ldflags" \
            KHDR_INCLUDES="$khdr_includes" \
            "$@" \
            "$output_dir/$target"
    ) >> "$build_log" 2>&1
}

kselftest_build_install_mm_collection() {
    local obj_root="$1"
    local cc_bin="$2"
    local build_log="$3"
    local install_root="$4"
    local out_dir="$obj_root/abi-parity-mm"
    local install_dir="$install_root/tests/mm"
    local khdr_includes="-isystem $obj_root/usr/include"
    local compat_include="$obj_root/abi-parity-include"
    local compat_lib="$obj_root/abi-parity-lib"

    mkdir -p "$out_dir" "$install_dir"
    kselftest_prepare_glibc_optional_deps "$compat_include" "$compat_lib"
    (
        cd "$ROOT/vendor/linux/tools/testing/selftests/mm"
        make \
            O="$obj_root" \
            OUTPUT="$out_dir" \
            INSTALL_PATH="$install_dir" \
            CC="$cc_bin" \
            ARCH=x86_64 \
            CAN_BUILD_I386=0 \
            CAN_BUILD_X86_64=1 \
            CAN_BUILD_WITH_NOPIE=0 \
            KHDR_INCLUDES="$khdr_includes" \
            USERCFLAGS="-I$compat_include" \
            USERLDFLAGS="-L$compat_lib" \
            -j "$JOBS" \
            all
        make \
            O="$obj_root" \
            OUTPUT="$out_dir" \
            INSTALL_PATH="$install_dir" \
            CC="$cc_bin" \
            ARCH=x86_64 \
            CAN_BUILD_I386=0 \
            CAN_BUILD_X86_64=1 \
            CAN_BUILD_WITH_NOPIE=0 \
            KHDR_INCLUDES="$khdr_includes" \
            USERCFLAGS="-I$compat_include" \
            USERLDFLAGS="-L$compat_lib" \
            install
    ) >> "$build_log" 2>&1
}

kselftest_prepare_glibc_optional_deps() {
    local include_dir="$1"
    local lib_dir="$2"
    local cap_lib numa_lib

    mkdir -p "$include_dir/sys" "$lib_dir"
    cat > "$include_dir/sys/capability.h" <<'EOF'
#ifndef ABI_PARITY_SYS_CAPABILITY_H
#define ABI_PARITY_SYS_CAPABILITY_H
typedef struct _cap_struct *cap_t;
cap_t cap_init(void);
int cap_set_proc(cap_t cap_p);
#endif
EOF
    cat > "$include_dir/numa.h" <<'EOF'
#ifndef ABI_PARITY_NUMA_H
#define ABI_PARITY_NUMA_H
#include <stddef.h>
struct bitmask {
    unsigned long size;
    unsigned long *maskp;
};
extern struct bitmask *numa_all_nodes_ptr;
int numa_available(void);
int numa_num_task_cpus(void);
int numa_max_possible_node(void);
int numa_max_node(void);
int numa_num_configured_nodes(void);
int numa_bitmask_isbitset(const struct bitmask *bmp, unsigned int n);
int numa_bitmask_weight(const struct bitmask *bmp);
long long numa_node_size(int node, long long *freep);
void *numa_alloc_onnode(size_t size, int node);
void numa_free(void *start, size_t size);
#endif
EOF
    cat > "$include_dir/numaif.h" <<'EOF'
#ifndef ABI_PARITY_NUMAIF_H
#define ABI_PARITY_NUMAIF_H
#include <sys/types.h>
#ifndef MPOL_MF_MOVE_ALL
#define MPOL_MF_MOVE_ALL (1 << 2)
#endif
int move_pages(pid_t pid, unsigned long count, void **pages,
               const int *nodes, int *status, int flags);
#endif
EOF

    cap_lib="$(ldconfig -p 2>/dev/null | awk '/libcap\.so\.2[[:space:]]/ { print $NF; exit }')"
    numa_lib="$(ldconfig -p 2>/dev/null | awk '/libnuma\.so\.1[[:space:]]/ { print $NF; exit }')"
    [[ -n "$cap_lib" && -f "$cap_lib" ]] && ln -sf "$cap_lib" "$lib_dir/libcap.so"
    [[ -n "$numa_lib" && -f "$numa_lib" ]] && ln -sf "$numa_lib" "$lib_dir/libnuma.so"
}

kselftest_stage_guest_runtime_deps() {
    local tests_root="$1"
    local lib_dir="$2"
    local elf dep base dst

    [[ -d "$tests_root" ]] || return 0
    mkdir -p "$lib_dir"
    while IFS= read -r -d '' elf; do
        if ! file "$elf" 2>/dev/null | grep -q 'ELF'; then
            continue
        fi
        while IFS= read -r dep; do
            [[ -n "$dep" && -f "$dep" ]] || continue
            base="${dep##*/}"
            case "$base" in
                ld-linux*.so*|libc.so*|libdl.so*|libm.so*|libpthread.so*|librt.so*|libresolv.so*)
                    continue
                    ;;
            esac
            dst="$lib_dir/$base"
            cp -L "$dep" "$dst"
            chmod 755 "$dst"
        done < <(
            ldd "$elf" 2>/dev/null \
                | sed -nE \
                    -e 's/.*=>[[:space:]]+(\/[^[:space:]]+).*/\1/p' \
                    -e 's/^[[:space:]]*(\/[^[:space:]]+)[[:space:]].*/\1/p'
        )
    done < <(find "$tests_root" -type f -perm /111 -print0)
}

kselftest_copy_guest_tree() {
    local install_root="$1"
    local guest_root="$2"
    local _initramfs_root="$3"
    mkdir -p "$guest_root"
    cp "$install_root/manifest.tsv" "$guest_root/manifest.tsv"
    cp "$install_root/run-kselftest-guest.sh" "$guest_root/run-kselftest-guest.sh"
    cp -a "$install_root/tests" "$guest_root/tests"
    kselftest_stage_guest_runtime_deps "$install_root/tests" "$guest_root/lib"
}

suite_dir() {
    printf '%s/%s\n' "$ARTIFACT_ROOT" "$1"
}

prepare_suite_dir() {
    local key="$1"
    local dir
    dir="$(suite_dir "$key")"
    mkdir -p "$dir"
    rm -rf "$dir/build" "$dir/guest-root"
    mkdir -p "$dir/build/install" "$dir/guest-root/etc" "$dir/lupos-results" "$dir/linux-ref" "$dir/compare"
    cat > "$dir/build/plan.env" <<EOF
suite=$key
artifact_root=$dir
guest_root=$dir/guest-root
lupos_results=$dir/lupos-results
linux_ref=$dir/linux-ref
compare_dir=$dir/compare
EOF
    cat > "$dir/guest-root/etc/abi-parity-suite.env" <<EOF
suite=$key
guest_results=/var/log/lupos/abi-parity/$key/summary.tsv
guest_raw=/var/log/lupos/abi-parity/$key/raw.log
EOF
}

write_blocker_manifest() {
    local key="$1"
    local test_id="$2"
    local note="$3"
    local blocker_path
    blocker_path="$(suite_dir "$key")/build/blocker.tsv"
    printf 'test_id\tnotes\n' > "$blocker_path"
    printf '%s\t%s\n' "$test_id" "$note" >> "$blocker_path"
}

suite_blocker_note() {
    local key="$1"
    case "$key" in
        ltp)
            if [[ ! -x "$ROOT/vendor/ltp/runltp" ]]; then
                printf '%s' "vendor/ltp is not synced locally; run cargo xtask vendor-sync --suite ltp --fetch"
            else
                printf '%s' "the curated LTP Linux/Lupos runner is not implemented yet"
            fi
            ;;
        xfstests)
            if [[ ! -x "$ROOT/vendor/xfstests/check" ]]; then
                printf '%s' "vendor/xfstests is not synced locally; run cargo xtask vendor-sync --suite xfstests --fetch"
            else
                printf '%s' "xfstests still needs ext4 test and scratch image provisioning plus a curated guest runner"
            fi
            ;;
        blktests)
            if [[ ! -x "$ROOT/vendor/blktests/check" ]]; then
                printf '%s' "vendor/blktests is not synced locally; run cargo xtask vendor-sync --suite blktests --fetch"
            else
                printf '%s' "blktests still needs virtio-blk image provisioning plus a curated guest runner"
            fi
            ;;
        packetdrill)
            if [[ ! -e "$ROOT/vendor/packetdrill/configure.ac" ]]; then
                printf '%s' "vendor/packetdrill is not synced locally; run cargo xtask vendor-sync --suite packetdrill --fetch"
            else
                printf '%s' "packetdrill still needs the curated TCP transcript harness wired into Linux and Lupos runs"
            fi
            ;;
        strace-ab)
            printf '%s' "strace-ab still needs a real Linux/Lupos workload runner wired into the fail-closed ABI parity suite path"
            ;;
        syzkaller)
            if ! command -v go >/dev/null 2>&1; then
                printf '%s' "host go toolchain is missing, so syzkaller cannot be built or launched"
            elif [[ ! -e "$ROOT/vendor/syzkaller/go.mod" ]]; then
                printf '%s' "vendor/syzkaller is not synced locally; run cargo xtask vendor-sync --suite syzkaller --fetch"
            else
                printf '%s' "syzkaller still needs the 7-day x 32-VM manager campaign wiring"
            fi
            ;;
        arch-rootfs)
            if command -v pacstrap >/dev/null 2>&1; then
                printf '%s' "pacstrap is available for Arch rootfs materialization"
            elif [[ ! -e "$ROOT/vendor/arch-rootfs/etc/os-release" ]]; then
                printf '%s' "vendor/arch-rootfs is not materialized locally; run cargo xtask vendor-sync --suite arch-rootfs --fetch"
            else
                printf '%s' "the Arch rootfs boot/package-manager guest runner is not implemented yet"
            fi
            ;;
        *)
            printf '%s' "ABI parity suite is still blocked by unresolved preflight prerequisites"
            ;;
    esac
}

build_preflight_blocker_suite() {
    local key="$1"
    local test_id="$2"
    prepare_suite_dir "$key"
    write_blocker_manifest "$key" "$test_id" "$(suite_blocker_note "$key")"
}

build_kselftest() {
    local key="kselftest"
    local dir manifest cc_bin obj_root install_root guest_root manifest_tsv build_log
    local entry collection test_name relpath out_dir
    local manifest_entry manifest_test_name manifest_relpath
    prepare_suite_dir "$key"
    need_path "vendor/linux/tools/testing/selftests"
    normalize_crlf_scripts "$ROOT/vendor/linux/scripts"
    normalize_crlf_scripts "$ROOT/vendor/linux/tools/testing/selftests"
    normalize_crlf_matching "$ROOT/vendor/linux/arch/x86/entry/syscalls" '*.tbl'
    manifest="$ROOT/src/docs/abi-parity-suite/manifests/kselftest-reference-set.txt"
    cp "$manifest" "$(suite_dir "$key")/build/reference-set.txt"
    obj_root="$(suite_dir "$key")/build/obj"
    install_root="$(suite_dir "$key")/build/install"
    guest_root="$(suite_dir "$key")/guest-root/opt/abi-parity-suite/kselftest"
    manifest_tsv="$install_root/manifest.tsv"
    build_log="$(suite_dir "$key")/build/kselftest-build.log"
    cc_bin="${CC:-gcc}"
    command -v "$cc_bin" >/dev/null 2>&1 || {
        echo "error: $cc_bin is required to build kselftest against glibc" >&2
        exit 1
    }

    rm -rf "$install_root/tests" "$guest_root"
    mkdir -p "$install_root/tests"
    : > "$build_log"
    printf 'test_id\tavailability\trelpath\tnotes\n' > "$manifest_tsv"

    make -C "$ROOT/vendor/linux" O="$obj_root" headers
    ensure_kselftest_header_compat "$obj_root/usr/include"
    kselftest_build_install_mm_collection "$obj_root" "$cc_bin" "$build_log" "$install_root"
    kselftest_stage_mm_support_files "$install_root"
    while IFS= read -r entry; do
        collection="${entry%%:*}"
        test_name="${entry#*:}"
        relpath="$collection/$test_name"
        manifest_entry="$entry"
        manifest_test_name="$test_name"
        manifest_relpath="$relpath"

        case "$entry" in
            mm:*.sh)
                if [[ -f "$install_root/tests/$manifest_relpath" ]]; then
                    kselftest_record_entry "$manifest_tsv" "$manifest_entry" "ready" "$manifest_relpath" "installed by upstream mm Makefile"
                else
                    echo "error: upstream mm Makefile did not install $manifest_entry" >&2
                    exit 1
                fi
                ;;
            mm:*)
                if [[ -f "$install_root/tests/$manifest_relpath" ]]; then
                    kselftest_record_entry "$manifest_tsv" "$manifest_entry" "ready" "$manifest_relpath" "built and installed by upstream mm Makefile"
                else
                    echo "error: upstream mm Makefile did not produce $manifest_entry" >&2
                    exit 1
                fi
                ;;
            sched:cs_prctl_test)
                out_dir="$obj_root/abi-parity-sched"
                if kselftest_try_build_target \
                    "$obj_root" "$cc_bin" "$build_log" "sched" "$out_dir" "cs_prctl_test" \
                    'USERCFLAGS=-D__GLIBC_PREREQ(x,y)=0'; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/cs_prctl_test"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from sched/cs_prctl_test"
                else
                    kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "sched/cs_prctl_test still assumes glibc-specific headers"
                fi
                ;;
            pidfd:pidfd_open_test)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "pidfd_open_test expects a tty-backed pidfd environment that the curated ABI parity runner does not provide"
                ;;
            futex:*)
                out_dir="$obj_root/abi-parity-futex"
                if kselftest_try_build_target \
                    "$obj_root" "$cc_bin" "$build_log" "futex/functional" "$out_dir" "$test_name" \
                    'LIBNUMA_TEST=NO'; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/$test_name"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from upstream futex/functional/$test_name"
                else
                    echo "error: upstream futex/functional/$test_name did not build against the current glibc harness" >&2
                    exit 1
                fi
                ;;
            timers:nanosleep)
                out_dir="$obj_root/abi-parity-timers"
                if kselftest_try_build_target "$obj_root" "$cc_bin" "$build_log" "timers" "$out_dir" "nanosleep"; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/nanosleep"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from timers/nanosleep"
                else
                    kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "timers/nanosleep did not build against the current glibc harness"
                fi
                ;;
            clone3:clone3)
                out_dir="$obj_root/abi-parity-clone3"
                if kselftest_try_build_target \
                    "$obj_root" "$cc_bin" "$build_log" "clone3" "$out_dir" "clone3" \
                    'LDLIBS='; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/clone3"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from clone3/clone3 without the cap helper target"
                else
                    kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "clone3 still requires host capability libraries in this environment"
                fi
                ;;
            ptrace:get_syscall_info)
                out_dir="$obj_root/abi-parity-ptrace"
                if kselftest_try_build_target "$obj_root" "$cc_bin" "$build_log" "ptrace" "$out_dir" "get_syscall_info"; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/get_syscall_info"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from ptrace/get_syscall_info"
                else
                    kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "ptrace/get_syscall_info did not build against the current glibc harness"
                fi
                ;;
            openat2:openat2_test)
                out_dir="$obj_root/abi-parity-openat2"
                if kselftest_try_build_target "$obj_root" "$cc_bin" "$build_log" "openat2" "$out_dir" "openat2_test"; then
                    kselftest_install_ready_artifact "$install_root" "$relpath" "$out_dir/openat2_test"
                    kselftest_record_entry "$manifest_tsv" "$entry" "ready" "$relpath" "built from openat2/openat2_test"
                else
                    kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "openat2/openat2_test still misses Linux UAPI headers under this glibc harness"
                fi
                ;;
            net:io_uring_zerocopy_tx.sh)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "io_uring net selftest still depends on host liburing and namespace tooling"
                ;;
            bpf:test_bpftool_build.sh)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "bpf selftests still require clang/libbpf host tooling outside the current ABI parity guest harness"
                ;;
            ftrace:ftracetest-ktap)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "ftrace selftests still expect tracefs/debugfs integration not staged in the ABI parity guest harness"
                ;;
            seccomp:seccomp_bpf)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "seccomp_bpf is outside the MM parity gate and still needs a curated runtime recipe"
                ;;
            landlock:base_test)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "landlock base_test is outside the MM parity gate and still needs a curated runtime recipe"
                ;;
            capabilities:test_execve)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "capabilities/test_execve is outside the MM parity gate and still needs a curated runtime recipe"
                ;;
            *)
                kselftest_record_entry "$manifest_tsv" "$entry" "skip" "-" "no curated build recipe is defined for this manifest entry yet"
                ;;
        esac
    done < <(manifest_entries "$manifest")

    write_abi_parity_kselftest_runner "$install_root/run-kselftest-guest.sh"
    kselftest_copy_guest_tree "$install_root" "$guest_root" "$(suite_dir "$key")/guest-root"
}

build_ltp() {
    local key="ltp"
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/ltp-families.txt" "ltp-families.txt"
}

build_xfstests() {
    local key="xfstests"
    local image_root
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/xfstests-auto-ext4.tsv" "xfstests-auto-ext4.tsv"
    image_root="$(suite_dir "$key")/build/images"
    stage_sparse_image "$image_root/test-ext4.img" "2G"
    stage_sparse_image "$image_root/scratch-ext4.img" "2G"
}

build_blktests() {
    local key="blktests"
    local image_root
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/blktests-virtio-blk.tsv" "blktests-virtio-blk.tsv"
    image_root="$(suite_dir "$key")/build/images"
    stage_sparse_image "$image_root/virtio-blk.raw" "8G"
}

build_packetdrill() {
    local key="packetdrill"
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/packetdrill-tcp-curated.txt" "packetdrill-tcp-curated.txt"
}

build_strace_ab() {
    local key="strace-ab"
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/strace-ab-workloads.tsv" "strace-ab-workloads.tsv"
}

build_syzkaller() {
    local key="syzkaller"
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/syzkaller-lupos-linux-reference.cfg" "lupos-linux-reference.cfg"
    mkdir -p "$(suite_dir "$key")/build/workdir" "$(suite_dir "$key")/build/images"
}

build_rootfs() {
    local key="$1"
    prepare_suite_dir "$key"
    stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/distro-packages.txt" "distro-packages.txt"
    case "$key" in
        arch-rootfs)
            stage_suite_manifest "$key" "src/docs/abi-parity-suite/manifests/arch-rootfs-smoke.tsv" "rootfs-smoke.tsv"
            ;;
        *)
            echo "error: unsupported ABI parity rootfs suite $key" >&2
            exit 1
            ;;
    esac
}

build_kernel_only_plan() {
    local key="$1"
    prepare_suite_dir "$key"
    need_path "vendor/linux"
}

build_one() {
    case "$1" in
        kselftest) build_kselftest ;;
        ltp) build_ltp ;;
        xfstests) build_xfstests ;;
        blktests) build_blktests ;;
        packetdrill) build_packetdrill ;;
        strace-ab) build_strace_ab ;;
        syzkaller) build_syzkaller ;;
        arch-rootfs) build_rootfs "arch-rootfs" ;;
        smp-preempt|smp-migration|pthread-smoke) build_kernel_only_plan "$1" ;;
        *)
            echo "error: unsupported ABI parity build suite $1" >&2
            exit 1
            ;;
    esac
}

if [[ "$suite" == "all" ]]; then
    for key in kselftest ltp xfstests blktests packetdrill strace-ab syzkaller arch-rootfs smp-preempt smp-migration pthread-smoke; do
        build_one "$key"
    done
else
    build_one "$suite"
fi

echo "ABI parity build/stage completed for $suite"
