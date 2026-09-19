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
* [CVE-2024-12086, CVE-2024-12087, and CVE-2024-12088](https://github.com/google/security-research/security/advisories/GHSA-p5pg-x43v-mvqj)
* [CVE-2024-12747](https://kb.cert.org/vuls/id/952657)
* [CVE-2025-31133](https://github.com/opencontainers/runc/security/advisories/GHSA-9493-h29p-rfm2)
* [CVE-2025-52565](https://github.com/opencontainers/runc/security/advisories/GHSA-qw9x-cqr3-wc7r)
* [CVE-2026-23954](https://github.com/lxc/incus/security/advisories/GHSA-7f67-crqm-jgh7)
* [CVE-2026-33711](https://github.com/lxc/incus/security/advisories/GHSA-q9vp-3wcg-8p4x)
* [CVE-2026-33897](https://github.com/lxc/incus/security/advisories/GHSA-83xr-5xxr-mh92)
* [CVE-2026-33945](https://github.com/lxc/incus/security/advisories/GHSA-q4q8-7f2j-9h9f)
* [CVE-2026-39860](https://github.com/NixOS/nix/security/advisories/GHSA-g3g9-5vj6-r3gj)
* [CVE-2026-29518 and CVE-2026-43619](https://github.com/RsyncProject/rsync/pull/895/changes/8471fdd1561049ef5f58df44a1811a50bd9a531d)
* [RedSun][] and [BlueHammer][] (2026) -- These vulnerabilities were in Windows
  (which libpathrs does not yet support) and usage of libpathrs itself would
  not have avoided the issue, but being forced to write file-handle-based code
  (which libpathrs does) would've avoided it.
* [CVE-2026-46703](https://github.com/boxlite-ai/boxlite/security/advisories/GHSA-f396-4rp4-7v2j)
* [CVE-2026-48749](https://github.com/lxc/incus/security/advisories/GHSA-2q3f-q5pq-g8wv)
* [CVE-2026-48750](https://github.com/lxc/incus/security/advisories/GHSA-73hr-m85f-64v9)
* [CVE-2026-48752](https://github.com/lxc/incus/security/advisories/GHSA-vxp5-584q-c479)
* [CVE-2026-48753](https://github.com/lxc/incus/security/advisories/GHSA-ccjc-4qc3-jxqc)
* [CVE-2026-48769](https://github.com/lxc/incus/security/advisories/GHSA-f6m5-xw2g-xc4x)
* [CVE-2026-53783](https://github.com/RsyncProject/rsync/security/advisories/GHSA-9cgc-64g4-3gv5)
* [CVE-2026-53784](https://github.com/RsyncProject/rsync/security/advisories/GHSA-ffg2-fr5g-3rxw)
* [CVE-2026-53785](https://github.com/RsyncProject/rsync/security/advisories/GHSA-pph3-7xmf-rrqg)
* [CVE-2026-53793](https://github.com/RsyncProject/rsync/security/advisories/GHSA-wj7w-vh23-mm44)
* [CVE-2026-53795](https://github.com/RsyncProject/rsync/security/advisories/GHSA-m9vj-637x-v6pq)
* [CVE-2026-53796](https://github.com/RsyncProject/rsync/security/advisories/GHSA-w75h-ccff-w53m)
* [CVE-2026-53797](https://github.com/RsyncProject/rsync/security/advisories/GHSA-3jj3-qvc7-jp6x)
* [CVE-2026-53799](https://github.com/RsyncProject/rsync/security/advisories/GHSA-phxh-hjqv-39c9)
* [CVE-2026-53800](https://github.com/RsyncProject/rsync/security/advisories/GHSA-v3vw-pvpg-chwh)
* [CVE-2026-53801](https://github.com/RsyncProject/rsync/security/advisories/GHSA-mch3-qr4p-chgm)
* [CVE-2026-53802](https://github.com/RsyncProject/rsync/security/advisories/GHSA-4mfr-8jrv-49x4)
* [CVE-2026-53803](https://github.com/RsyncProject/rsync/security/advisories/GHSA-g9f4-7q66-9582)
* [CVE-2026-63125](https://github.com/lxc/incus/security/advisories/GHSA-6rqx-22hc-qm36)
* [CVE-2026-63343](https://github.com/lxc/incus/security/advisories/GHSA-fmjx-5j3g-997p)
* [CVE-2026-70460](https://github.com/RsyncProject/rsync/security/advisories/GHSA-w3xf-j2r2-gv4x)
* [CVE-2026-81493](https://github.com/lxc/incus/security/advisories/GHSA-p2v3-6wvc-cv3p)
* [CVE-2026-81494](https://github.com/lxc/incus/security/advisories/GHSA-7fj9-65v4-rp7h)
* [CVE-2026-81495](https://github.com/lxc/incus/security/advisories/GHSA-67qw-68v3-36h6)
* [CVE-2026-81496](https://github.com/lxc/incus/security/advisories/GHSA-26gp-p5fw-3r2h)
* [CVE-2026-81497](https://github.com/lxc/incus/security/advisories/GHSA-4qxq-p5hm-3q3p)
* [CVE-2026-81500](https://github.com/lxc/incus/security/advisories/GHSA-9pqw-c7m4-xvg7)
* [CVE-2026-85706](https://hackerone.com/reports/3909881)

[RedSun]: https://web.archive.org/web/20260521144507/https://github.com/Nightmare-Eclipse/RedSun
[BlueHammer]: https://web.archive.org/web/20260406225410/https://github.com/Nightmare-Eclipse/BlueHammer

### [Strict Path Safety](./procfs-api.md) ###

These bugs are more specific than "classic" path safety, and usually involve a
privileged process operating on pseudofilesystems like `/proc` in a context
where an attacker may be able to modify the mount table of the process. This is
primarily a container-runtime-specific issue and most people probably consider
protecting against this to be a paranoid level of hardening.

 * [CVE-2019-16884](https://github.com/advisories/GHSA-fgv8-vj5c-2ppq)
 * [CVE-2019-19921](https://github.com/opencontainers/runc/security/advisories/GHSA-fh74-hm69-rqjw)
 * [CVE-2025-52881](https://github.com/opencontainers/runc/security/advisories/GHSA-cgrx-mc8f-2prm)
