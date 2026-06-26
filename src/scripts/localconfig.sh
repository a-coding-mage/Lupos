#!/usr/bin/env bash
# make localconfig — configure the Lupos kernel for THIS host, Linux-style.
#
# Mirrors the intent of Linux `make localmodconfig`/`localyesconfig`: probe the
# running host (lspci / ACPI / DMI) and write a tuned `.config` seeded from the
# generic x86_64 defconfig, then sync the generated `rustc_cfg` that
# `build.rs::emit_linux_rustc_cfg` consumes. Follow with `make build`.
#
# The host probe + Kconfig wiring already lives in localmodconfig.sh; localconfig
# is the user-facing single entry point for "configure for this machine" and
# delegates to it so the detection matrix has one source of truth.
set -euo pipefail

DEFCONFIG="${DEFCONFIG:-configs/lupos_defconfig}"
here="$(cd "$(dirname "$0")" && pwd)"

echo "*** localconfig: probing this host and seeding from ${DEFCONFIG}"
echo "*** localconfig: virtio/PCI drivers are emitted as modules (=m); Lupos loads"
echo "*** localconfig: Linux-built .ko payloads rather than reimplementing drivers"

# Delegate host detection + .config/syncconfig to the shared implementation.
DEFCONFIG="${DEFCONFIG}" exec bash "${here}/localmodconfig.sh"
