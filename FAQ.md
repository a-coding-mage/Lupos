# Lupos FAQ

## Why does Lupos exist?

Lupos aims to be a Kernel/OS primarily AI-made in Rust that replicates the
Linux ABI for x86_64 first, with AArch64 planned later. The goal is binary-level
Linux compatibility with a kernel implementation written in Rust where possible.

The default image uses a minimal Arch `base` userland so regular Linux tools can
be exercised inside the guest. The single, non-negotiable success metric is
**100% binary-level parity with the Linux kernel ABI**.

## Is this serious?

Mostly a meme. It started as a "what if we rewrote Linux in safe Rust with AI"
experiment, and the name reflects that.

That said, the contract is straightforward: the Linux experience does not
change. Same ABI, same syscalls, same `/proc`, same `/sys`, same ioctls. The
kernel internals can change, but observable Linux behavior cannot.

## What should I expect today?

- Builds a pure kernel with `make kernel`.
- Builds a bootable ISO with a minimal Arch `base` userland using `make image`.
- Uses one shared generic x86_64 config derived from Linux's `x86_64_defconfig`.
- Boots through GRUB with `root` / `lupos` as the default login.
- Provides baseline QEMU devices: framebuffer/DRM video, 8250 serial,
  virtio-net user networking, xHCI USB with a tablet, and Intel HDA audio.
- Exercises ICMP/DNS smoke paths through `cargo xtask run --ping-smoke`.
- Tracks remaining ABI work in `ROADMAP.md`.

Do not expect a daily driver. Do expect something you can boot, inspect, and
contribute to.

## Is this a fork of Linux?

No. Lupos is a rewrite, not a refactor. There is no Linux C code in the kernel
itself. `vendor/linux/` is included as the source of truth for ABI behavior,
struct layouts, errno values, and selftests.

## Why Rust?

Two reasons:

1. **Safety.** The ownership and borrow model eliminates many use-after-free,
   double-free, and data-race bugs at compile time.
2. **Modern tooling.** Cargo, integrated tests, and a strong type system make
   it easier for contributors and AI agents to reason about kernel code than C.
3. I just woke up and chose violence for fun.

Unsafe Rust is used where hardware or ABI boundaries require it. ABI fidelity
to `vendor/linux/` always overrides Rust ergonomics.

## Will my Linux software still work?

That is the point. If a binary runs on Linux x86_64, it should run on Lupos with
identical observable behavior. Any divergence is treated as a bug. That includes
syscall return values, `errno`, struct layouts, signal semantics, `/proc` and
`/sys` contents, and ioctl behavior.

If an x86_64 binary does not run, capture logs and screenshots and open an
issue with the failing command.

## Why "Lupos"?

This comes from a joke with my name, when I found out the meaning and origin of
"Lopes". It is meant to be the cursed sibling of Linux: same ABI, different
implementation.

## What platforms are supported?

- **Now:** x86_64 on native Linux hosts.
- **Planned:** AArch64.

QEMU is the primary target for boot and tests.

## How do I run it?

See [README.md](README.md). The short version:

```bash
make config
make image
cargo xtask run
```

Log in as `root` / `lupos`.

For a graphical login instead of the text console, run `cargo xtask run --gui`
(or `make run-gui`). It boots into a LightDM GTK greeter on the framebuffer;
logging in as `root` / `lupos` starts the XFCE desktop.

## Why don't I hear the terminal bell (`printf '\a'`)?

The kernel does emit the bell: a ground-state BEL (`0x07`) drives the emulated
PC speaker (PIT channel 2 gated through port `0x61`), exactly like Linux's VT
`bell()` / `pcspkr` path. Whether you *hear* it depends on the VM routing that
speaker to host audio.

- **QEMU** (`cargo xtask run`): the PC speaker is wired to the shared audiodev
  with `-machine ...,pcspk-audiodev=luposaudio`, but the default audio backend
  is the null sink (`none`) so headless/CI runs stay silent. To actually hear
  it, pick a host backend:

  ```bash
  LUPOS_QEMU_AUDIODEV=pa cargo xtask run        # PulseAudio
  LUPOS_QEMU_AUDIODEV=pipewire cargo xtask run  # PipeWire
  ```

- **VirtualBox** (`luposbox`): enable PC-speaker passthrough once, then
  power-cycle the VM:

  ```bash
  scripts/lupos-vbox-beep.sh            # mode 1: host PC speaker
  scripts/lupos-vbox-beep.sh luposbox 2 # mode 2: route via host audio device
  ```

Inside the guest, test with `printf '\a'` (note: `prinf` in the screenshotted
session was a typo, and `printf '\a'` with an unterminated quote waits for more
input — close the quote).

## How do I contribute?

1. Pick an unfinished milestone in `ROADMAP.md` or open a pull request based on
   an issue.
2. Read the relevant Linux source under `vendor/linux/`; it is the source of
   truth for behavior.
3. Build or port the Linux test first, or add a local test only when the
   behavior is Lupos-specific.
4. Implement.
5. Run the boot/unit tests and relevant original Linux tests.
6. Update `ROADMAP.md` and collapse the milestone to the compact completed
   format.

Test-driven development is mandatory. No implementation without a test.

## Why AI-made?

Because writing a Linux-parity kernel by hand is a multi-decade effort. Lupos
is an experiment in how far AI-driven development can be pushed on a hard,
well-specified problem with a clear oracle: Linux itself.

## Is this competing with Linux?

No. Lupos depends on Linux being the reference. If Linux changes its behavior,
Lupos follows. The goal is parity, not divergence.

## What is the worst-case outcome?

It stays a meme and a learning project.

## What is the best-case outcome?

A useful story, a lot of Linux knowledge, and maybe a safer kernel underneath
an unchanged Linux ABI.
