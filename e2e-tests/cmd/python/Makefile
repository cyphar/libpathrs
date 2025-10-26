#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

define script =
#!/bin/bash
# SPDX-License-Identifier: MPL-2.0
#
# libpathrs: safe path resolution on Linux
# Copyright (C) 2019-2025 Aleksa Sarai <cyphar@cyphar.com>
# Copyright (C) 2019-2025 SUSE LLC
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

set -Eeuo pipefail

SRC_DIR="$(readlink -f "$(dirname "${BASH_SOURCE[0]}")")"

exec "$SRC_DIR/.venv/bin/python3" "$SRC_DIR/pathrs-cmd.py" "$@"
endef

.ONESHELL:

.PHONY: pathrs-cmd
pathrs-cmd: .venv
	cat >$@ <<'EOF'
	$(value script)
	EOF
	chmod +x $@

.PHONY: .venv
.venv:
	python3 -m venv $@
	make -C ../../../contrib/bindings/python
	$@/bin/pip3 install --force ../../../contrib/bindings/python/dist/*.whl
