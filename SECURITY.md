# Security policy

Lupos is pre-production research software. It is not a supported security
boundary and currently has no stable security release or guaranteed response
window.

## Safe testing

- Use a disposable VM and a non-sensitive host.
- Do not attach host disks, home-directory shares, credentials or production
  networks.
- Do not expose the guest directly to the Internet.
- Assume guest compromise, data corruption, denial of service and incorrect
  permission enforcement are possible.
- Keep the hypervisor and host kernel patched; Lupos cannot protect the host
  from hypervisor vulnerabilities.

The published image uses the public development credential `root` / `lupos`.
It must never be treated as a secure default.

## Reporting

For non-sensitive bugs, open a GitHub issue with a minimal reproducer, exact
commit, command and serial log.

Do not publish a working exploit, secret, or embargoed vulnerability in an
issue. Use GitHub's private vulnerability reporting feature for this repository
if it is available. If it is not available, contact the repository owner
privately through the contact method on their GitHub profile before sharing
details. Remove real credentials and personal data from every report.

## Scope

Security reports are useful, but until the project declares otherwise there are
no production-supported versions, no CVE assignment commitment and no
backport policy. Fixes normally land on the main development branch.
