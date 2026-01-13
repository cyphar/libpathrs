# SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 SUSE LLC
# Copyright (C) 2026 Aleksa Sarai <cyphar@cyphar.com>
#
# == MPL-2.0 ==
#
#  This Source Code Form is subject to the terms of the Mozilla Public
#  License, v. 2.0. If a copy of the MPL was not distributed with this
#  file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Alternatively, this Source Code Form may also (at your option) be used
# under the terms of the GNU Lesser General Public License Version 3, as
# described below:
#
# == LGPL-3.0-or-later ==
#
#  This program is free software: you can redistribute it and/or modify it
#  under the terms of the GNU Lesser General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or (at
#  your option) any later version.
#
#  This program is distributed in the hope that it will be useful, but
#  WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
#  or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
#  for more details.
#
#  You should have received a copy of the GNU Lesser General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

CARGO ?= cargo
CARGO_NIGHTLY ?= cargo +nightly

# Unfortunately --all-features needs to be put after the subcommand, but
# cargo-hack needs to be put before the subcommand. So make a function to make
# this a little easier.
ifneq (, $(shell which cargo-hack))
define cargo_hack
	$(1) hack --feature-powerset $(2)
endef
else
define cargo_hack
	$(1) $(2) --all-features
endef
endif
CARGO_CHECK := $(call cargo_hack,$(CARGO),check)
CARGO_CLIPPY := $(call cargo_hack,$(CARGO),clippy)
CARGO_LLVM_COV := $(call cargo_hack,$(CARGO_NIGHTLY),llvm-cov)

RUSTC_FLAGS := --features=capi -- -C panic=abort -C linker=clang -C link-arg=-fuse-ld=lld
CARGO_FLAGS ?=

SRC_FILES = $(wildcard Cargo.*) $(shell find . -name '*.rs')

.DEFAULT: debug
.PHONY: debug
debug: target/debug

target/debug: $(SRC_FILES)
	# For some reason, --crate-types needs separate invocations. We can't use
	# #![crate_type] unfortunately, as using it with #![cfg_attr] has been
	# deprecated. <https://github.com/rust-lang/rust/issues/91632>
	$(CARGO) rustc $(CARGO_FLAGS) --crate-type=cdylib    $(RUSTC_FLAGS)
	$(CARGO) rustc $(CARGO_FLAGS) --crate-type=staticlib $(RUSTC_FLAGS)

.PHONY: release
release: target/release

target/release: $(SRC_FILES)
	# For some reason, --crate-types needs separate invocations. We can't use
	# #![crate_type] unfortunately, as using it with #![cfg_attr] has been
	# deprecated. <https://github.com/rust-lang/rust/issues/91632>
	$(CARGO) rustc $(CARGO_FLAGS) --release --crate-type=cdylib    $(RUSTC_FLAGS)
	$(CARGO) rustc $(CARGO_FLAGS) --release --crate-type=staticlib $(RUSTC_FLAGS)

.PHONY: smoke-test
smoke-test:
	make -C examples smoke-test

.PHONY: clean
clean:
	-rm -rf target/

.PHONY: lint
lint: validate-cbindgen lint-rust

.PHONY: lint-rust
lint-rust:
	$(CARGO_NIGHTLY) fmt --all -- --check
	$(CARGO_CLIPPY) --all-targets
	$(CARGO_CHECK) $(CARGO_FLAGS) --all-targets

.PHONY: validate-cbindgen
validate-cbindgen:
	$(eval TMPDIR := $(shell mktemp --tmpdir -d libpathrs-cbindgen-check.XXXXXXXX))
	@																		\
		trap "rm -rf $(TMPDIR)" EXIT ;										\
		cbindgen -c cbindgen.toml -o $(TMPDIR)/pathrs.h ;					\
		if ! ( diff -u include/pathrs.h $(TMPDIR)/pathrs.h ); then			\
			echo -e															\
			  "\n"															\
			  "ERROR: include/pathrs.h is out of date.\n\n"					\
			  "Changes to the C API in src/capi/ usually need to be paired with\n" \
			  "an update to include/pathrs.h using cbindgen. To fix this error,\n" \
			  "just run:\n\n"												\
			  "\tcbindgen -c cbindgen.toml -o include/pathrs.h\n" ;			\
			exit 1 ;														\
		 fi

.PHONY: test-rust-doctest
test-rust-doctest:
	$(CARGO_LLVM_COV) --no-report --branch --doc

.PHONY: test-rust-unpriv
test-rust-unpriv:
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)"
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)" --enosys=openat2
	# In order to avoid re-running the entire test suite with just statx
	# disabled, we re-run the key procfs tests with statx disabled.
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)" --enosys=statx "test(#tests::*procfs*)"

.PHONY: test-rust-root
test-rust-root:
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)" --sudo
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)" --sudo --enosys=openat2
	# In order to avoid re-running the entire test suite with just statx
	# disabled, we re-run the key procfs tests with statx disabled.
	./hack/rust-tests.sh --cargo="$(CARGO_NIGHTLY)" --sudo --enosys=statx "test(#tests::*procfs*)"

.PHONY: test-rust
test-rust:
	-rm -rf target/llvm-cov*
	make test-rust-{doctest,unpriv,root}

.PHONY: test-e2e
test-e2e:
	make -C e2e-tests test-all
	make -C e2e-tests RUN_AS=root test-all

.PHONY: test
test: test-rust test-e2e
	$(CARGO_NIGHTLY) llvm-cov report
	$(CARGO_NIGHTLY) llvm-cov report --open

.PHONY: docs
docs:
	$(CARGO) doc --all-features --document-private-items --open

.PHONY: install
install: release
	@echo "If you want to configure the install paths, use ./install.sh directly."
	@echo "[Sleeping for 3 seconds.]"
	@sleep 3s
	./install.sh
