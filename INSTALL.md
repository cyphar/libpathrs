# Installing libpathrs #

At the moment, libpathrs is not packaged for most distributions:

| Library Packaging Status | Rust Crate Packaging Status |
|:------------------------:|:---------------------------:|
|![Library Packaging Status](https://repology.org/badge/vertical-allrepos/libpathrs.svg?header=libpathrs)|![Rust Crate Packaging Status](https://repology.org/badge/vertical-allrepos/rust:pathrs.svg?header=rust:pathrs)|

and so it is often necessary to build and install it from source.

## Downloading a Release ##

Official release tarballs are available from [our releases page][releases].
Each release contains:

- A source tarball (`libpathrs-$version.tar.xz`) with a detached GPG signature
  (`.asc`).
- A vendored copy of the Rust dependencies (`libpathrs.vendor.tar.zst`) with a
  detached GPG signature (`.asc`).
- A (legacy) inline clear-signed checksums file (`libpathrs.sha256sum`).

[releases]: https://github.com/cyphar/libpathrs/releases

### Verifying the Release Signature ###

All official releases are signed with a key present in [`libpathrs.keyring`][keyring]
in the source tree. If this is your first time verifying a release, you can get
the keyring from the [source repository][libpathrs-src] or from a previously
verified release tarball.

[keyring]: ./libpathrs.keyring
[libpathrs-src]: https://github.com/cyphar/libpathrs

```bash
$ # Import the libpathrs release keyring into a temporary keyring.
$ tmp_gpgdir="$(mktemp -d)"
$ gpg --homedir="$tmp_gpgdir" --no-default-keyring \
      --keyring=libpathrs.keyring --import libpathrs.keyring

$ # Verify individual artefact signatures.
$ gpg --homedir="$tmp_gpgdir" --no-default-keyring \
      --keyring=libpathrs.keyring \
      --verify libpathrs-$version.tar.xz.asc libpathrs-$version.tar.xz
```

## Build Dependencies ##

libpathrs is written in Rust and requires a working Rust toolchain. The minimum
supported Rust version (MSRV) is **1.63**, though we recommend using the latest
stable release.

### Rust Toolchain ###

Most Linux distributions ship `cargo` and `rustc` packages:

| Distribution       | Command                           |
| ------------------ | --------------------------------- |
| Debian / Ubuntu    | `apt install cargo rustc`         |
| Fedora             | `dnf install cargo rust`          |
| openSUSE           | `zypper install cargo rust`       |
| Arch Linux         | `pacman -S rust`                  |

Distribution-packaged Rust versions may be quite old -- make sure yours ships
at least Rust 1.63. If it doesn't, use [rustup][] instead:

```bash
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installing, make sure `~/.cargo/bin` is in your `$PATH`.

[rustup]: https://rustup.rs/

### Other Build Dependencies ###

You will also need GNU `make`, `clang`, and `lld`.

| Distribution       | Command                                    |
| ------------------ | ------------------------------------------ |
| Debian / Ubuntu    | `apt install build-essential clang lld`    |
| Fedora             | `dnf install make clang lld`               |
| openSUSE           | `zypper install make clang lld`            |
| Arch Linux         | `pacman -S base-devel clang lld`           |

## Building ##

```bash
$ tar xf libpathrs-$version.tar.xz
$ cd libpathrs-$version
$ make release
```

This produces both `libpathrs.so` and `libpathrs.a` under `target/release/`.

On some architectures, `rustc` will include additional linker flags that are
not supported by `lld`. In order to work around this issue, you may need to
explicitly specify `clang` as the linker driver:

```bash
$ make release EXTRA_RUSTC_FLAGS="-C linker=clang"
```

### Vendored / Offline Builds ###

If you have the vendored dependencies tarball (for offline builds), extract it
and point Cargo at it:

```bash
$ tar xvf libpathrs.vendor.tar.zst
$ mkdir -p .cargo
$ cat >.cargo/config.toml <<EOF
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF
$ make release
```

## Installing ##

`install.sh` is a fairly minimal installation script that accepts a subset of
autoconf-style arguments and generates a corresponding `pkg-config` manifest
that is also installed. This should be sufficient for most users, but very
specialised installation environments may need to come up with their own
installation methods.

```bash
$ ./install.sh --prefix=/usr --libdir=/usr/lib64
```

For distribution packaging with a staging root:

```bash
$ ./install.sh DESTDIR=$BUILDROOT \
      --prefix=/usr --libdir=/usr/lib64
```

`make install` is just shorthand for `./install.sh --prefix=/usr/local` and we
recommend you use `install.sh` directly.

If installing onto your host system, you should run `sudo ldconfig` after
installation so the link loader will pick up the new library.

### `lib` vs `lib64` ###

`install.sh` tries to autodetect whether to install into `lib` or `lib64`
under the exec-prefix by checking whether a `lib64` directory exists on the
host system. This heuristic doesn't always get it right -- some systems have a
dummy `lib64` that is not included in the paths handled by `ldconfig`, and the
check for `lib64` ignores `DESTDIR` so cross-distribution `install.sh`
invocations are likely to break. You should always pass `--libdir` explicitly
if there's any doubt about the correct path for your distribution:

- **Fedora, openSUSE, RHEL** (64-bit): `/usr/lib64`
- **Debian, Ubuntu, Arch Linux**: `/usr/lib` (or `/usr/lib/x86_64-linux-gnu`
  on multiarch Debian/Ubuntu)

### `install.sh` Options ###

| Flag               | Default                   | Description                               |
| ------------------ | ------------------------- | ----------------------------------------- |
| `--prefix`         | `/usr/local`              | Installation prefix                       |
| `--exec-prefix`    | `$PREFIX`                 | Executable prefix                         |
| `--includedir`     | `$PREFIX/include`         | Header file directory                     |
| `--libdir`         | `$EPREFIX/lib` or `lib64` | Library directory (autodetected)          |
| `--pkgconfigdir`   | `$LIBDIR/pkgconfig`       | pkg-config `.pc` file directory           |
| `--rust-target`    | native                    | Rust target triple (for cross-compilation)|
| `--rust-buildmode` | `release`                 | Rust build profile to install from        |
| `DESTDIR`          | `/`                       | Staging root (for distribution packaging) |

## Python Bindings ##

The Python bindings for `libpathrs` are [available on PyPI][pypi]:

```bash
pip install pathrs
```

[pypi]: https://pypi.org/project/pathrs/

If you need to build the bindings from source (for distribution packaging or
development), read on.

### Building from Source ###

The bindings live in `contrib/bindings/python/` and use [CFFI][] in API mode to
link against `libpathrs.so`. You'll need Python >= 3.9, `cffi`, `setuptools`,
and the `build` frontend:

[CFFI]: https://cffi.readthedocs.io/

| Distribution       | Command                                                          |
| ------------------ | ---------------------------------------------------------------- |
| Debian / Ubuntu    | `apt install python3-dev python3-cffi python3-build python3-wheel` |
| Fedora             | `dnf install python3-devel python3-cffi python3-build python3-wheel` |
| openSUSE           | `zypper install python3-devel python3-cffi python3-build python3-wheel` |
| Arch Linux         | `pacman -S python python-cffi python-build python-wheel`         |

If `libpathrs.so` and `pathrs.h` are already installed on the system, building
is straightforward:

```bash
$ cd contrib/bindings/python
$ python3 -m build
```

#### `PATHRS_SRC_ROOT` ####

When packaging for a distribution, it may be more natural to build the Python
bindings in the same build script as the main `libpathrs.so` library, meaning
that `pathrs*.whl` will be built without `libpathrs.so` having been installed
on the host. As a workaround for this, you can set `PATHRS_SRC_ROOT` to point
at the libpathrs source tree (which must have been already compiled):

```bash
$ # Build libpathrs.so first.
$ make release

$ # Then build the Python bindings against the source tree.
$ cd contrib/bindings/python
$ PATHRS_SRC_ROOT=/path/to/libpathrs python3 -m build
```

This tells the CFFI build script to find `pathrs.h` in
`$PATHRS_SRC_ROOT/include/` and `libpathrs.so` in
`$PATHRS_SRC_ROOT/target/{debug,release}/` (it prefers whichever was built more
recently). Without `PATHRS_SRC_ROOT`, the build assumes a system-wide install
and will fail if the header or library isn't found.

### Installing ###

```bash
$ pip3 install contrib/bindings/python/dist/pathrs*.whl
```

`make -B contrib/bindings/python install` is shorthand for the above command.

At runtime, `libpathrs.so` must be in the dynamic linker search path (installed
system-wide via `ldconfig`, or reachable via `LD_LIBRARY_PATH`).

## Using libpathrs ##

### C ###

Use `pkg-config` to get the right compiler and linker flags:

```bash
gcc -o myprogram myprogram.c $(pkg-config --cflags --libs pathrs)
```

Or in a `Makefile`:

```make
CFLAGS  += $(shell pkg-config --cflags pathrs)
LDFLAGS += $(shell pkg-config --libs pathrs)
```

### Python ###

```python
import pathrs

with pathrs.Root("/path/to/rootfs") as root:
    with root.resolve("/etc/passwd") as handle:
        with handle.reopen("r") as f:
            for line in f:
                print(line.rstrip("\n"))
```
