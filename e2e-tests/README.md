## `libpathrs` End-to-End Binding Tests ##

In addition to [the extensive Rust-based tests][rust-tests] we have (which test
our Rust implementation as well as the C binding wrappers from Rust), we also
have verification tests to ensure that our language bindings have the correct
behaviour.

These tests are not designed to be as extensive as our Rust tests (which test
the behaviour of `libpathrs` itself), but instead are just intended to verify
that the language bindings are correctly wrapping the right `libpathrs` APIs.

[rust-tests]: ../src/tests/

### Running ###

```console
$ make -C e2e-tests test-all               # all bindings
$ make -C e2e-tests test-{python,go,rust}  # individual bindings
```

By default, all bindings are tested against the `libpathrs.so` built from the
source tree. For bindings that support linking against the host's
`libpathrs.so` (i.e., all but the Rust ones) it is possible to test them
against the host's `libpathrs.so` by setting `PATHRS_E2E_USE_HOST_SO=1`:

```console
$ make -C e2e-tests PATHRS_E2E_USE_HOST_SO=1 test-all
```
