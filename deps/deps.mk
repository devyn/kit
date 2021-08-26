################################################################################
#
# kit/deps/deps.mk
# - build rules for all of kit's dependencies
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015-2021, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

include deps/system_deps.mk

build/deps/.dir: build/.dir
	mkdir -p build/deps
	touch build/deps/.dir

all-deps: all-system-deps

clean-deps: clean-system-deps

clean-dep-sources:
	rm -rf deps/rust

.PHONY: all-deps clean-deps clean-dep-sources
