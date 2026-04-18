### Strict Path Safety (for `procfs`) ###

`libpathrs` also provides a set of primitives to safely interact with `procfs`.
This is very important for some programs (such as container runtimes), because
`/proc` has several key system administration purposes that make it different
to other filesystems. It particular, `/proc` is used:

1. As a mechanism for doing certain filesystem operations through
   `/proc/self/fd/...` (and other similar magic-links) that cannot be done by
   other means.
1. As a source of true information about processes and the general system (such
   as by looking `/proc/$pid/status`).
1. As an administrative tool for managing processes (such as setting LSM labels
   like `/proc/self/attr/apparmor/exec`).

These operations have stronger requirements than regular filesystems. For (1)
we need to open the magic-link for real (magic-links are symlinks that are not
resolved lexically, they are in-kernel objects that warp you to other files
without doing a regular path lookup) which much harder to do safely (even with
`openat2`). For (2) and (3) we have the requirement that we need to open a
specific file, not just any file within `/proc` (if there are overmounts or
symlinks) which is not the case `pathrs_inroot_resolve()`. As a result, it is
necessary to take far more care when doing operations of `/proc` and
`libpathrs` provides very useful helper to do this. Failure to do so can lead
to security issues such as those in [CVE-2019-16884][cve-2019-16884] and
[CVE-2019-19921][cve-2019-19921].

In addition, with the [new mount API][lwn-newmount] ([`fsopen(2)`] and
[`open_tree(2)`] in particular, added in Linux 5.2), it is possible to get a
totally private `procfs` handle that can be used without worrying about racing
mount operations. `libpathrs` will try to use this if it can (this usually
requires root).

[cve-2019-16884]: https://nvd.nist.gov/vuln/detail/CVE-2019-16884
[cve-2019-19921]: https://nvd.nist.gov/vuln/detail/CVE-2019-19921
[lwn-newmount]: https://lwn.net/Articles/759499/
[`open_tree(2)`]: https://www.man7.org/linux/man-pages/man2/open_tree.2.html
[`fsopen(2)`]: https://www.man7.org/linux/man-pages/man2/fsopen.2.html

### Examples ###

The most common usage of the procfs API is writing to specific files in
`/proc`. The following are two examples that are adapted from real code in
container runtimes:

<table>
<tr><th>Rust</th><th>C</th><th>Go</th></tr>
<tr>
<td>

```rust
use pathrs::{
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle},
};

/// Configure the current process's AppArmor label.
fn write_apparmor_label(label: impl AsRef<[u8]>) -> Result<(), Error> {
    ProcfsHandle::new()?
        .open(
            ProcfsBase::ProcSelf,
            "attr/apparmor/exec",
            // You should always use O_NOFOLLOW unless you are dealing with
            // magic-links or otherwise really need to operate on a trailing
            // symlink.
            OpenFlags::O_WRONLY | OpenFlags::O_NOFOLLOW,
        )?
        .write_all(label.as_ref())?;
    Ok(())
}
```
```rust
use std::path::Path;

use pathrs::{
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle},
};

/// Configure the given sysctl.
fn write_sysctl(name: impl AsRef<str>, value: impl AsRef<[u8]>) -> Result<(), Error> {
    // /proc/sys/<key/name>
    let key_path = Path::new("sys").join(name.as_ref().replace(".", "/"));
    ProcfsHandle::new()?
        .open(
            ProcfsBase::ProcRoot,
            key_path,
            OpenFlags::O_WRONLY | OpenFlags::O_NOFOLLOW,
        )?
        .write_all(value.as_ref())?;
    // If you need to do lots of these operations, you should use
    // ProcfsHandleBuilder::unmasked() to create a temporary handle.
    Ok(())
}
```

</td>
<td>

```c
#include <unistd.h>
#include <fcntl.h>
#include <string.h>

#include <pathrs.h>

/*
 * Safely set the AppArmor exec label for the current process. This is
 * something runc does while configuring the container process.
 */
int write_apparmor_label(const char *label)
{
    int fd, err;

    fd = pathrs_proc_open(PATHRS_PROC_SELF, "attr/apparmor/exec",
                          O_WRONLY|O_NOFOLLOW);
    if (IS_PATHRS_ERR(fd)) {
        pathrs_error_t *error = pathrs_errorinfo(fd);
        /* ... print the error ... */
        pathrs_errorinfo_free(error);
        return -1;
    }

    err = write(fd, label, strlen(label));
    close(fd);
    return err;
}
```
```c
#define _GNU_SOURCE
#include <unistd.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>

#include <pathrs.h>

/*
 * Safely configure the given sysctl.
 */
int write_sysctl(const char *name, const char *value)
{
    int fd, err;
    char *path = NULL;

    if (asprintf(&path, "sys/%s", name))
        return -1;

    for (char *p = path; *p; p++)
        if (*p == '.')
            *p = '/';

    fd = pathrs_proc_open(PATHRS_PROC_ROOT, path,
                          O_WRONLY|O_NOFOLLOW);
    free(path);
    if (IS_PATHRS_ERR(fd)) {
        pathrs_error_t *error = pathrs_errorinfo(fd);
        /* ... print the error ... */
        pathrs_errorinfo_free(error);
        return -1;
    }

    err = write(fd, value, strlen(value));
    close(fd);
    return err;
}
```

</td>
<td>

```go
import (
    "cyphar.com/go-pathrs/procfs"
    "golang.org/x/sys/unix"
)

// Configure the current process's AppArmor label.
func writeAppArmorLabel(label string) error {
    proc, err := procfs.Open()
    if err != nil {
        return err
    }
    defer proc.Close()

    // You should always use O_NOFOLLOW unless you are dealing with
    // magic-links or otherwise really need to operate on a trailing
    // symlink.
    file, err := proc.OpenSelf("attr/apparmor/exec", unix.O_WRONLY|unix.O_NOFOLLOW)
    if err != nil {
        return err
    }
    defer file.Close()

    _, err = file.WriteString(label)
    return err
}
```
```go
import (
    "strings"

    "cyphar.com/go-pathrs/procfs"
    "golang.org/x/sys/unix"
)

// Configure the given sysctl.
func writeSysctl(name, value string) error {
    proc, err := procfs.Open()
    if err != nil {
        return err
    }
    defer proc.Close()

    // /proc/sys/<key/name>
    keyPath := "sys/" + strings.ReplaceAll(name, ".", "/")
    file, err := proc.OpenRoot(keyPath, unix.O_WRONLY|unix.O_NOFOLLOW)
    if err != nil {
        return err
    }
    defer file.Close()

    _, err = file.WriteString(value)
    // If you need to do lots of these operations, you should use
    // procfs.Open(procfs.UnmaskedProcRoot) to create a temporary handle.
    return err
}
```

</td>
</tr>
</table>

Another very powerful primitive is safe magic-link operations, which allow you
to operate on certain files with guarantees from the kernel that the object is
what you expect. Note that this operation (opening a magic-link) is the hardest
for us to secure and thus your security relies on you having privileges to be
able to call [`fsopen(2)`].

[`fsopen(2)`]: https://www.man7.org/linux/man-pages/man2/fsopen.2.html

<table>
<tr><th>Rust</th><th>C</th><th>Go</th></tr>
<tr>
<td>

```rust
use std::fs::File;

use pathrs::{
    flags::OpenFlags,
    procfs::{ProcfsBase, ProcfsHandle},
};

/// Safely get an fd to /proc/self/exe.
fn get_self_exe() -> Result<File, Error> {
    let file = ProcfsHandle::new()?.open(
        ProcfsBase::ProcSelf,
        "exe",
        OpenFlags::O_PATH, // no O_NOFOLLOW!
    )?;
    Ok(file)
}
```

</td>
<td>

```c
#include <unistd.h>
#include <fcntl.h>

#include <pathrs.h>

/*
 * Safely get an fd to /proc/self/exe. This is something runc does to re-exec
 * itself during the container setup process.
 */
int get_self_exe(void)
{
    /* This follows the trailing magic-link! */
    int fd = pathrs_proc_open(PATHRS_PROC_SELF, "exe", O_PATH);
    if (IS_PATHRS_ERR(fd)) {
        pathrs_error_t *error = pathrs_errorinfo(fd);
        /* ... print the error ... */
        pathrs_errorinfo_free(error);
        return -1;
    }
    return fd;
}
```

</td>
<td>

```go
import (
    "os"

    "cyphar.com/go-pathrs/procfs"
    "golang.org/x/sys/unix"
)

// Safely get an fd to /proc/self/exe. This is something runc
// does to re-exec itself during the container setup process.
func getSelfExe() (*os.File, error) {
    proc, err := procfs.Open()
    if err != nil {
        return nil, err
    }
    defer proc.Close()

    // This follows the trailing magic-link -- no O_NOFOLLOW!
    return proc.OpenSelf("exe", unix.O_PATH)
}
```

</td>
</tr>
</table>

In some (rare) cases, you need to get the "real" path for a file descriptor.
This path MUST NOT be used for actual filesystem operations because it's
possible for an attacker to move the file or change one of the path components
to a symlink, which could lead to you operating on files you didn't expect
(including host files). Similarly, you should not use the pathname for
permissive security decisions because the attacker can always rename the file.

<table>
<tr><th>Rust</th><th>C</th><th>Go</th></tr>
<tr>
<td>

```rust
use std::{
    os::fd::{AsFd, AsRawFd},
    path::PathBuf,
};

use pathrs::procfs::{ProcfsBase, ProcfsHandle};

/// Get an *unstable* and *unsafe* path string for a file descriptor.
fn get_unsafe_path(fd: impl AsFd) -> Result<PathBuf, Error> {
    let path = ProcfsHandle::new()?.readlink(
        ProcfsBase::ProcThreadSelf,
        format!("fd/{}", fd.as_fd().as_raw_fd()),
    )?;
    Ok(path)
}
```

</td>
<td>

```c
#define _GNU_SOURCE
#include <unistd.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <pathrs.h>

/*
 * Get an *unstable* and *unsafe* path string for a file descriptor.
 *
 * In most cases, this kind of function would be used for diagnostic purposes
 * (such as in error messages, to provide context about what file the error is
 * in relation to).
 */
char *get_unsafe_path(int fd)
{
    char *fdpath;

    if (asprintf(&fdpath, "fd/%d", fd) < 0)
        return NULL;

    int linkbuf_size = 128;
    char *linkbuf = malloc(size);
    if (!linkbuf)
        goto err;
    for (;;) {
        int len = pathrs_proc_readlink(PATHRS_PROC_THREAD_SELF,
                                       fdpath, linkbuf, linkbuf_size);
        if (IS_PATHRS_ERR(len)) {
            pathrs_error_t *error = pathrs_errorinfo(fd);
            /* ... print the error ... */
            pathrs_errorinfo_free(error);
            goto err;
        }

        if (len <= linkbuf_size)
            break;

        linkbuf_size = len;
        linkbuf = realloc(linkbuf, linkbuf_size);
        if (!linkbuf)
            goto err;
    }

    free(fdpath);
    return linkbuf;

err:
    free(fdpath);
    free(linkbuf);
    return NULL;
}
```

</td>
<td>

```go
import (
    "fmt"

    "cyphar.com/go-pathrs/procfs"
)

// Get an *unstable* and *unsafe* path string for a file descriptor.
//
// In most cases, this kind of function would be used for diagnostic
// purposes (such as in error messages, to provide context about what
// file the error is in relation to).
func getUnsafePath(fd int) (string, error) {
    proc, err := procfs.Open()
    if err != nil {
        return "", err
    }
    defer proc.Close()

    return proc.Readlink(procfs.ProcThreadSelf, fmt.Sprintf("fd/%d", fd))
}
```

</td>
</tr>
</table>
