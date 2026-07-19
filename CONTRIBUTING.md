# Contributing to Lupos

Lupos accepts fixes, tests, documentation and compatibility work. This is an
experimental kernel, so a small change can have ABI-wide consequences. Keep
pull requests focused and make the Linux behavior being reproduced easy to
verify.

## Ground rules

1. The current target is generic x86_64.
2. Observable behavior and ABI must match the corresponding Linux source.
   A different implementation is acceptable only where Rust cannot reproduce
   the C implementation directly; externally visible behavior must still match.
3. Kernel core work is Rust. Original Linux C drivers are built separately and
   loaded through Lupos's module/KAPI compatibility layer.
4. Prefer original Linux KUnit, kselftest or subsystem tests. Add a local test
   only for Lupos-specific build or integration behavior.
5. Do not weaken an existing test, parity gate or security check to make a
   change pass.
6. Be explicit about partial behavior. Never mark a source-parity unit
   `complete` without verifying the entire claimed unit.

All contributions are submitted under [GPL-2.0-only](LICENSE).

## Set up

Follow the host setup in the [README](README.md#build-and-reproduce), then:

```bash
./vendor/setup_linux.sh
make config
cargo xtask build
cargo xtask test
```

Do not commit `vendor/linux`, build artifacts, `.config`, root disks, ISO
images or downloaded package caches.

## Workflow

1. Identify the exact Linux source file and version used as the oracle.
2. Identify or port the original Linux test that observes the behavior.
3. Implement the smallest coherent compatibility unit.
4. Compare layouts, constants, return values, `errno`, ordering, lifetime and
   concurrency behavior with Linux.
5. Run formatting, the default test gate and the narrow QEMU/upstream test for
   the changed subsystem.
6. Document any behavior that is still partial or untested.

Useful gates:

```bash
cargo fmt --all -- --check
cargo xtask test
cargo xtask test --boot
cargo xtask test --mode module-loader
cargo xtask test --mode userspace-smoke
cargo xtask test --all
```

`--all` is expensive. Run the narrowest relevant boot mode during development
and the broader gate before requesting merge when practical.

## Pull request checklist

- The description names the Linux source and test used as evidence.
- The change preserves x86_64 ABI layouts and observable behavior.
- Unsafe Rust and assembly have a documented invariant.
- Error and cleanup paths are tested, not just the success path.
- New public behavior is reflected in the README/FAQ where appropriate.
- No generated disks, ISOs, caches, credentials or secrets are included.
- Material AI assistance is disclosed.

## AI-assisted contributions

AI tools are allowed and are central to this experiment, but the submitter
remains responsible for every line.

In the pull request, name the tools/models used when known and summarize what
they did: for example source translation, test generation, debugging,
documentation or review. Also describe the human verification performed.
Do not present AI-produced reasoning, parity tags or passing local unit tests as
proof of Linux parity. Verify against `vendor/linux` and original Linux tests.

Never paste private code, secrets, embargoed vulnerabilities or data you do not
have permission to share into an external model.

## Reporting bugs

Include the commit, host distribution, `rustc -Vv`, QEMU version, exact command,
serial log and the smallest reproducer. For security-sensitive findings, follow
[SECURITY.md](SECURITY.md) and do not attach secrets or exploit material to a
public issue.
