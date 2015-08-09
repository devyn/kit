################################################################################
#
# kit/deps/deps.mk
# - build rules for all of kit's dependencies
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

include deps/kernel_deps.mk
include deps/system_deps.mk

build/deps/.dir: build/.dir
	mkdir -p build/deps
	touch build/deps/.dir

all-deps: deps/rust/.dir all-kernel-deps all-system-deps

clean-deps: clean-kernel-deps clean-system-deps

clean-dep-sources:
	rm -rf deps/rust

deps/rust/.dir:
	git clone --depth 1 https://github.com/rust-lang/rust.git deps/rust
	touch deps/rust/.dir

.PHONY: all-deps clean-deps clean-dep-sources
