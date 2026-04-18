## Kernel Feature Support ##

`libpathrs` was designed alongside [`openat2(2)`][] (available since Linux 5.6)
and dynamically tries to use the latest kernel features to provide the maximum
possible protection against racing attackers. However, it also provides support
for older kernel versions (in theory up to Linux 2.6.39 but we do not currently
test this) by emulating newer kernel features in userspace.

However, we strongly recommend you use at least Linux 5.6 to get a
reasonable amount of protection against various attacks, and ideally at
least Linux 6.8 to make use of all of the protections we have implemented.

See the following table for what kernel features we optionally support and
what they are used for.

| Feature                 | Minimum Kernel Version  | Description | Fallback |
| ----------------------- | ----------------------- | ----------- | -------- |
| `/proc/thread-self`     | Linux 3.17 (2014-10-05) | Used when operating on the current thread's `/proc` directory for use with `PATHRS_PROC_THREAD_SELF`. | `/proc/self/task/$tid` is used, but this might not be available in some edge cases so `/proc/self` is used as a final fallback. |
| [`open_tree(2)`][]      | Linux 5.2 (2019-07-07)  | Used to create a private procfs handle when operating on `/proc` (this is a copy of the host `/proc` -- in most cases this will also strip any overmounts). Requires `CAP_SYS_ADMIN` privileges. | Open a regular handle to `/proc`. This can lead to certain race attacks if the attacker can dynamically create mounts. |
| [`fsopen(2)`][]         | Linux 5.2 (2019-07-07)  | Used to create a private procfs handle when operating on `/proc` (with a completely fresh copy of `/proc` -- in some cases this operation will fail if there are locked overmounts on top of `/proc`). Requires `CAP_SYS_ADMIN` privileges. | Try to use [`open_tree(2)`] instead -- in the case of errors due to locked overmounts, [`open_tree(2)`] will be used to create a recursive copy that preserves the overmounts. This means that an attacker would not be able to actively change the mounts on top of `/proc` but there might be some overmounts that libpathrs will detect (and reject). |
| [`openat2(2)`][]        | Linux 5.6 (2020-03-29)  | In-kernel restrictions of path lookup. This is used extensively by `libpathrs` to safely do path lookups. | Userspace emulated path lookups. |
| `subset=pid`            | Linux 5.8 (2020-08-02)  | Allows for a `procfs` handle created with [`fsopen(2)`][] to not contain any global procfs files that would be dangerous for an attacker to write to. Detached `procfs` mounts with `subset=pid` are deemed safe(r) to leak into containers and so libpathrs will internally cache `subset=pid` `ProcfsHandle`s. | libpathrs's `ProcfsHandle`s will have global files and thus libpathrs will not cache a copy of the file descriptor for each operation (possibly causing substantially higher syscall usage as a result -- our testing found that this can have a performance impact in some cases). |
| `STATX_MNT_ID`          | Linux 5.8 (2020-08-02)  | Used to verify whether there are bind-mounts on top of `/proc` that could result in insecure operations (on systems with `fsopen(2)` or `open_tree(2)` this protection is somewhat redundant for privileged programs -- those kinds of `procfs` handles will typically not have overmounts.) | Parse the `/proc/thread-self/fdinfo/$fd` directly -- for systems with `openat2(2)`, this is guaranteed to be safe against attacks. For systems without `openat2(2)`, we have to fallback to unsafe opens that could be fooled by bind-mounts -- however, we believe that exploitation of this would be difficult in practice (even with an attacker that has the persistent ability to mount to arbitrary paths) due to the way we verify `procfs` accesses. |
| `ino` field in `fdinfo` | Linux 5.14 (2021-08-29) | Used to harden the emulation of `RESOLVE_NO_XDEV` with`fdinfo` if `openat2(2)` and `STATX_MNT_ID` are blocked or otherwise unavailable. | **None**. We do still check the `mnt_id` field but because the value is static and attacker-known, in princple an attacker could (via a somewhat complicated attack) overmount fake `fdinfo` files to hide a mountpoint from `libpathrs`. |
| `STATX_MNT_ID_UNIQUE`   | Linux 6.8 (2024-03-10)  | Used for the same reason as `STATX_MNT_ID`, but allows us to protect against mount ID recycling. This is effectively a safer version of `STATX_MNT_ID`. | `STATX_MNT_ID` is used (see the `STATX_MNT_ID` fallback if it's not available either). |

For more information about the work behind `openat2(2)`, you can read the
following LWN articles (note that the merged version of `openat2(2)` is
different to the version described by LWN):

 * [New AT_ flags for restricting pathname lookup][lwn-atflags]
 * [Restricting path name lookup with openat2()][lwn-openat2]

[`openat2(2)`]: https://www.man7.org/linux/man-pages/man2/openat2.2.html
[`open_tree(2)`]: https://www.man7.org/linux/man-pages/man2/open_tree.2.html
[`fsopen(2)`]: https://www.man7.org/linux/man-pages/man2/fsopen.2.html
[lwn-atflags]: https://lwn.net/Articles/767547/
[lwn-openat2]: https://lwn.net/Articles/796868/

