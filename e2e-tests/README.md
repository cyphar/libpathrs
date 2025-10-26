## `libpathrs` End-to-End Binding Tests ##

In addition to [the extensive Rust-based tests][rust-tests] we have (which test
our Rust implementation as well as the C binding wrappers from Rust), we also
have verification tests to ensure that our language bindings have the correct
behaviour.

These tests are not designed to be as extensive as our Rust tests (which test
the behaviour of `libpathrs` itself), but instead are just intended to verify
that the language bindings are correctly wrapping the right `libpathrs` APIs.

[rust-tests]: ../src/tests/
