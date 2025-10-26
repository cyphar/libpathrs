## `libpathrs` Binding-Agnostic Test Binary API ##

In order to allow us to test the full functionality of the `libpathrs` bindings
in a uniform way, we need have some kind of binary-agnostic API that the tests
can use so we can test the same functionality for all bindings easily.
This is accomplished by implementing a test binary for each binding that
implements a fairly simple CLI interface that tests can be written against.

The basic usage of the tool looks like this:

```
pathrs-cmd <operation> <operation args...>
```

The following operations are all part of the API used by our tests in
[`e2e-tests/tests`](../tests). Any change in this API should result in changes
in the test suite.

#### Error Output ####

Each section below describes the `pathrs-cmd` output in successful cases, but
if an operation fails the output will be consistent:

```
ERRNO <errno code> (<errno string>)
ERROR-DESCRIPTION <error message>
```

Note that `<errno string>` can have different formatting based on the language,
so tests should only ever test against the numerical `<errno code>`. Tests
against `<error message>` are acceptable but should be used carefully as such
tests could be brittle.

### `Root` Operations ###

All of the following operations are subcommands of the `root` subcommand.

#### `resolve` ####

```
pathrs-cmd root --root <root>
           resolve [--[no-]follow] [--reopen=<oflags>] <subpath>
```

Calls `Root::resolve` (or `Root::resolve_nofollow`) with the given `subpath`
argument.

 * `--no-follow` indicates to use `Root::resolve_nofollow` (i.e., do not follow
   trailing symlinks) rather than `Root::resolve`. The default is `--follow`.
 * `--reopen` indicates that we should re-open the handle with the given set of
   `O_*` flags.

##### Output #####

```
HANDLE-PATH <full path of resolved handle>
FILE-PATH <full path of reopened file from handle>
```

**TODO**: We currently do not do an I/O for the re-opened file.

#### `open` ####

```
pathrs-cmd root --root <root>
           open [--[no-]follow] [--oflags=<oflags=O_RDONLY>] <subpath>
```

Calls `Root::open_subpath` with the given `subpath` argument.

 * `--oflags` is the set of `O_*` flags to use when opening the subpath.
 * `--no-follow` is equivalent to `--oflags O_NOFOLLOW`. The default is
   `--follow`.
<!--
 * `--no-follow` is generally equivalent to `--oflags O_NOFOLLOW` but some
   bindings might switch between using `Root::open` and `Root::open_follow`
   directly based on this flag instead. In general, bindings that have a
   distinction should treat `--[no-]follow` as calling different APIs, while
   `--oflags` should only affect the set of `O_*` flags passed.
 -->

##### Output #####

```
FILE-PATH <full path of opened file>
```

**TODO**: We currently do not do an I/O for the opened file.

#### `mkfile` ####

```
pathrs-cmd root --root <root>
           mkfile [--oflags=<oflags=O_RDONLY>] [--mode <mode=0o644>] <subpath>
```

Calls `Root::create_file` with the given `subpath` argument. This is
effectively a safe `O_CREAT|O_EXCL|O_NOFOLLOW`.

 * `--oflags` is the set of `O_*` flags to use when creating the file.
   (`O_CREAT` is not required and is actually an invalid argument.)
 * `--mode` is the mode of the created file (umask still applies).

##### Output #####

```
FILE-PATH <full path of opened file>
```

**TODO**: We currently do not do an I/O for the newly-created file.

#### `mkdir` ####

```
pathrs-cmd root --root <root>
           mkdir [--mode <mode=0o755>] <subpath>
```

Calls `pathrs_inroot_mkdir` (`Root::create(InodeType::Directory)`) with the
given `subpath` argument.

 * `--mode` is the mode of the created directory (umask still applies).

#### `mkdir-all` ####

```
pathrs-cmd root --root <root>
           mkdir-all [--mode <mode=0o755>] <subpath>
```

Calls `Root::mkdir_all` with the given `subpath` argument.

 * `--mode` is the mode of the created directories (umask still applies).
   Existing directories do not have their modes changed.

##### Output #####

```
HANDLE-PATH <full path of deepest created directory>
```

#### `mknod` ####

```
pathrs-cmd root --root <root>
           mknod [--mode <mode=0o644>] <subpath> <type> [<major=0> <minor=0>]
```

Calls `Root::create` with various inode types with the given `subpath`
argument, loosely effectively equivalent to `mknod(1)`.

 * `--mode` is the mode of the created inode (umask still applies).
 * `type` is the inode type to create, and must be one of the following values:
   - `f`: regular **f**ile
   - `d`: **d**irectory
   - `b`: **b**lock device (`major:minor`)
   - `c` (or `u`): **c**character device (`major:minor`)
   - `p`: named **p**ipe (aka FIFO)
 * `major` and `minor` only have effect for `b` and `c`/`u` but `pathrs-cmd`
   will pass the device value to all calls if they are specified.

#### `hardlink` ####

```
pathrs-cmd root --root <root> hardlink <target> <linkname>
```

Calls `Root::create(InodeType::Hardlink)` with the given `subpath` argument.
Both `target` and `linkname` are resolved inside the root.

 * `target` is an existing path that will be the target of the new hardlink.
 * `linkname` is the path to where the new hardlink will be placed.

Note that the argument order is the same as `ln(1)`!

#### `symlink` ####

```
pathrs-cmd root --root <root> symlink <target> <linkname>
```

Calls `Root::create(InodeType::Symlink)` with the given `subpath` argument.

 * `target` is an arbitrary string that will be the content of the symlink.
 * `linkname` is the path to where the new symlink will be placed.

Note that the argument order is the same as `ln(1)`!

#### `readlink` ####

```
pathrs-cmd root --root <root> readlink <subpath>
```

Calls `Root::readlink` with the given `subpath` argument.

##### Output #####

```
LINK-TARGET <link target>
```

#### `rmdir` ####

```
pathrs-cmd root --root <root> rmdir <subpath>
```

Calls `Root::remove_dir` with the given `subpath` argument. (`subpath` needs to
be an empty directory.)

#### `unlink` ####

```
pathrs-cmd root --root <root> unlink <subpath>
```

Calls `Root::remove_file` with the given `subpath` argument. (`subpath` needs
to be a non-directory.)

#### `remove-all` ####

```
pathrs-cmd root --root <root> remove-all <subpath>
```

Calls `Root::remove_all` with the given `subpath` argument.

#### `rename` ####

```
pathrs-cmd root --root <root>
           rename [--whiteout] [--exchange] [--[no-]clobber]
           <source> <destination>
```

Calls `Root::rename` with the given `source` and `destination` arguments.

 * `--whiteout` indicates that the `RENAME_WHITEOUT` flag should be set.
 * `--exchange` indicates that the `RENAME_EXCHANGE` flag should be set.
 * `--no-clobber` indicates the the `RENAME_NOREPLACE` flag should be set.

Some of these flag combinations are not permitted by Linux, however
`pathrs-cmd` allows any combination to be provided and for libpathrs to return
an error if appropriate.

### `ProcfsHandle` Operations ###

All of the following operations are subcommands of the `procfs` subcommand. All
procfs commands take the following arguments:

 * `--unmasked` indicates whether to use `ProcfsHandleBuilder::unmasked()` when
   configuring the `ProcfsHandle`. At the moment there isn't really a way to
   determine that this is not a no-op in our tests, but we may expand this
   capability in the future (such as by looking at `mnt_id` after successive
   calls).

 * `--base` indicates what `ProcfsBase` to use, and must be one of the
   following values:
   - `root`: `ProcfsBase::ProcRoot`
   - `self`: `ProcfsBase::ProcSelf`
   - `thread-self`: `ProcfsBase::ProcThreadSelf`
   - `pid=$n`: `ProcfsBase::ProcPid($n)` (`$n` is an integer)
   The default is `--base root`.

**TODO**: We should probably expose `ProcfsHandleRef::try_from_fd`.

#### `open` ####

```
pathrs-cmd procfs [--unmasked] [--base <base>]
           open [--[no-]follow] [--oflags <oflags=O_RDONLY>] <subpath>`
```

Calls `ProcfsHandle::open` (or `ProcfsHandle::open_follow`) with the given
`subpath` argument.

 * `--oflags` is the set of `O_*` flags to use when opening the subpath.
 * `--no-follow` is generally equivalent to `--oflags O_NOFOLLOW` but some
   bindings might switch between using `ProcfsHandle::open` and
   `ProcfsHandle::open_follow` directly based on this flag instead. In general,
   bindings that have a distinction should treat `--[no-]follow` as calling
   different APIs, while `--oflags` should only affect the set of `O_*` flags
   passed. (libpathrs treats both equivalently, but for test purposes we keep
   this distinction to make sure that libpathrs bindings also treat these
   equivalently.) The default is `--follow`.

##### Output #####

```
FILE-PATH <full path of opened file>
```

**TODO**: We currently do not do an I/O for the opened file.

#### `readlink` ####

```
pathrs-cmd procfs [--unmasked] [--base <base>]
           readlink <subpath>`
```

Calls `ProcfsHandle::readlink` with the given `subpath` argument.

##### Output #####

```
LINK-TARGET <link target>
```
