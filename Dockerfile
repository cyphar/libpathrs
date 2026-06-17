# SPDX-License-Identifier: MPL-2.0 OR LGPL-3.0-or-later
#
# libpathrs: safe path resolution on Linux
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

ARG DEBIAN_RELEASE=trixie
ARG RUST_VERSION=1.96

# --------------------------------------------------------------------------- #
# build: builds libpathrs for use by CI and the "install" image.
# --------------------------------------------------------------------------- #
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS build

RUN apt-get update -y && \
    apt-get upgrade -y && \
    apt-get install -y --no-install-recommends \
        clang \
        lld \
        make \
        pkg-config && \
    apt-get clean -y && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/libpathrs
COPY . /usr/src/libpathrs
RUN make release && \
    DESTDIR=/opt/libpathrs ./install.sh --prefix=/usr --libdir=/usr/lib

# ----------------------------------------------------------------------------
# install: minimal runtime image with libpathrs installed system-wide.
# Intended to be used as a base image by downstream projects on distros that do
# not ship a libpathrs package yet.
# ----------------------------------------------------------------------------
FROM debian:${DEBIAN_RELEASE} AS install

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update -y && \
    apt-get upgrade -y && \
    apt-get install -y --no-install-recommends \
        pkg-config && \
    apt-get clean -y && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /opt/libpathrs/ /
# debian doesn't use /usr/lib for the native architecture so we need to make
# sure it gets searched by the link loader with ldconfig.
RUN ldconfig

# ----------------------------------------------------------------------------
# ci: full test runner for CI and local test runs.
# This can run the Rust unit/integration tests and the e2e tests.
# ----------------------------------------------------------------------------
ARG RUST_VERSION=1.96
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS ci

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
        bats \
        curl \
        clang \
        git \
        golang-go \
        jq \
        lld \
        llvm \
        moreutils \
        python3 \
        python3-build \
        python3-dev \
        python3-pip \
        python3-setuptools \
        python3-venv \
        sudo && \
    apt-get clean -y && \
    rm -rf /var/lib/apt/lists/*

# Use a globally-writable place for Go caches.
ENV GOCACHE=/tmp/go-cache/build
ENV GOMODCACHE=/tmp/go-cache/mod

ARG CARGO_BINSTALL_VERSION=1.19.1
RUN CARGO_BINSTALL_VERSION="$CARGO_BINSTALL_VERSION" \
        curl -L --proto '=https' --tlsv1.2 -sSf \
            "https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v$CARGO_BINSTALL_VERSION/install-from-binstall-release.sh" | bash

ARG CARGO_LLVM_COV_VERSION=0.8.7
ARG CARGO_HACK_VERSION=0.6.45
ARG CARGO_NEXTEST_VERSION=0.9.137
RUN cargo binstall --no-confirm \
        "cargo-llvm-cov@$CARGO_LLVM_COV_VERSION" \
        "cargo-hack@$CARGO_HACK_VERSION" \
        "cargo-nextest@$CARGO_NEXTEST_VERSION"

ARG RUST_NIGHTLY=nightly-2026-06-03
RUN rustup toolchain install "$RUST_NIGHTLY" && \
    rustup component add llvm-tools llvm-tools-preview && \
    rustup component add --toolchain "$RUST_NIGHTLY" llvm-tools llvm-tools-preview
ENV CARGO_NIGHTLY="cargo +$RUST_NIGHTLY"

# We want the installed libpathrs library for the Python and Go tests.
COPY --from=build /opt/libpathrs/ /
# Debian doesn't use /usr/lib for the native architecture so we need to make
# sure it gets searched by the link loader with ldconfig.
RUN ldconfig

WORKDIR /usr/src/libpathrs
COPY . /usr/src/libpathrs

# Populate the cache for test runs and make sure the ownership is friendly for
# non-root.
FROM ci AS ci-with-cache
RUN cargo test --workspace --all-features --no-run && \
    $CARGO_NIGHTLY llvm-cov --workspace --doc --all-features --no-report && \
    find "$CARGO_HOME" /usr/src/libpathrs -type d -print0 | xargs -0 -P$(nproc) chmod a+rwx && \
    find "$CARGO_HOME" /usr/src/libpathrs -type f -print0 | xargs -0 -P$(nproc) chmod a+rw
