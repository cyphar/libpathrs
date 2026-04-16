## List of Avoidable Vulnerabilities ##

The following is a list of vulnerabilities that we believe would've been
avoided if the project in question had used libpathrs for file operations.
For more information about the "strict" and "classic" path safety terms, [see
my FOSDEM 2026 talk on the topic][fosdem2026-path-safety].

This is not intended to speak ill of other projects (almost all languages
provide substandard APIs for doing VFS operations and this is not an issue most
people consider), but is instead intended to provide a justification for why
this library exists and provides C APIs to maximise adoption.

If you know of any other issues that libpathrs would've protected against, [feel
free to open a PR](https://github.com/cyphar/libpathrs/edit/main/docs/avoidable-vulnerabilities.md)!

[fosdem2026-path-safety]: https://fosdem.org/2026/schedule/event/H39QZD-path_safety_in_the_trenches/

### Classic Path Safety ###

These bugs were related to classic symlink traversal or similar
time-of-check-time-of-use bugs. Most Unix programs are at risk of having bugs
of this nature, and so we anticipate this list is much longer than given here.

* [docker/docker#5656](https://github.com/moby/moby/issues/5656) (2014)
* [CVE-2017-1002101](https://github.com/kubernetes/kubernetes/issues/60813)
* [CVE-2018-15664](https://www.openwall.com/lists/oss-security/2019/05/28/1)
* [CVE-2019-16884](https://github.com/advisories/GHSA-fgv8-vj5c-2ppq)
* [CVE-2019-19921](https://github.com/opencontainers/runc/security/advisories/GHSA-fh74-hm69-rqjw)
* [CVE-2021-30465](https://github.com/opencontainers/runc/security/advisories/GHSA-c3xm-pvg7-gh7r)
* [CVE-2023-27561](https://github.com/opencontainers/runc/security/advisories/GHSA-g2j6-57v7-gm8c)
* [CVE-2023-28642](https://github.com/opencontainers/runc/security/advisories/GHSA-g2j6-57v7-gm8c)
* [CVE-2024-1753](https://github.com/containers/podman/security/advisories/GHSA-874v-pj72-92f3)
* [CVE-2024-45310](https://github.com/opencontainers/runc/security/advisories/GHSA-jfvp-7x6p-h2pv)
* [CVE-2024-0132](https://github.com/NVIDIA/libnvidia-container/security/advisories/GHSA-q2v4-jw5g-9xxj)
* [CVE-2024-0133](https://github.com/NVIDIA/libnvidia-container/security/advisories/GHSA-xff4-h7r9-vrpf)
* [CVE-2024-9676](https://github.com/advisories/GHSA-wq2p-5pc6-wpgf)
* [CVE-2025-31133](https://github.com/opencontainers/runc/security/advisories/GHSA-9493-h29p-rfm2)
* [CVE-2025-52565](https://github.com/opencontainers/runc/security/advisories/GHSA-qw9x-cqr3-wc7r)
* [CVE-2026-33711](https://github.com/lxc/incus/security/advisories/GHSA-q9vp-3wcg-8p4x)
* [CVE-2026-33897](https://github.com/lxc/incus/security/advisories/GHSA-83xr-5xxr-mh92)
* [CVE-2026-33945](https://github.com/lxc/incus/security/advisories/GHSA-q4q8-7f2j-9h9f)
* [CVE-2026-39860](https://github.com/NixOS/nix/security/advisories/GHSA-g3g9-5vj6-r3gj)

### [Strict Path Safety](./procfs-api.md) ###

These bugs are more specific than "classic" path safety, and usually involve a
privileged process operating on pseudofilesystems like `/proc` in a context
where an attacker may be able to modify the mount table of the process. This is
primarily a container-runtime-specific issue and most people probably consider
protecting against this to be a paranoid level of hardening.

 * [CVE-2019-16884](https://github.com/advisories/GHSA-fgv8-vj5c-2ppq)
 * [CVE-2019-19921](https://github.com/opencontainers/runc/security/advisories/GHSA-fh74-hm69-rqjw)
 * [CVE-2025-52881](https://github.com/opencontainers/runc/security/advisories/GHSA-cgrx-mc8f-2prm)
