#!/usr/bin/env bash
# Enable PC-speaker passthrough for the luposbox VirtualBox VM.
#
# Lupos drives the emulated PC speaker (PIT channel 2 + port 0x61) when the
# console receives a BEL (0x07), e.g. `printf '\a'`. VirtualBox discards that
# speaker output unless passthrough is explicitly enabled via the internal
# i8254 device config, so the host stays silent without this one-time setup.
#
# Ref: VirtualBox manual, "9.x Tuning / PC speaker passthrough".
#
# Usage:
#   scripts/lupos-vbox-beep.sh [VM_NAME] [MODE]
#
#   VM_NAME  VirtualBox VM name (default: luposbox)
#   MODE     PassthroughSpeaker value (default: 1)
#              1 = host PC speaker (kernel console beep; needs host-console
#                  access, the classic "real" beep)
#              2 = route the speaker through the host's default audio device
#                  (use this when there is no usable physical PC speaker)
#
# The VM must be powered off when this runs; the setting takes effect on the
# next boot.
set -euo pipefail

VM_NAME="${1:-luposbox}"
MODE="${2:-1}"
KEY="VBoxInternal/Devices/i8254/0/Config/PassthroughSpeaker"

if ! command -v VBoxManage >/dev/null 2>&1; then
    echo "VBoxManage not found on PATH; install VirtualBox first." >&2
    exit 1
fi

if ! VBoxManage showvminfo "$VM_NAME" >/dev/null 2>&1; then
    echo "VirtualBox VM '$VM_NAME' not found." >&2
    echo "Create/import it first, then re-run: $0 $VM_NAME $MODE" >&2
    exit 1
fi

VBoxManage setextradata "$VM_NAME" "$KEY" "$MODE"
echo "Enabled PC-speaker passthrough (mode $MODE) for '$VM_NAME'."
echo "Power-cycle the VM, then test inside the guest with:  printf '\\a'"
