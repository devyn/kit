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

LUA_VERSION=5.3.1
LUA_URL="http://www.lua.org/ftp/lua-${LUA_VERSION}.tar.gz"

build/deps/.dir: build/.dir
	mkdir -p build/deps
	touch build/deps/.dir

all-deps: deps/rust/.dir deps/lua/.dir all-kernel-deps all-system-deps

clean-deps: clean-kernel-deps clean-system-deps

clean-dep-sources:
	rm -rf deps/rust
	rm -rf deps/lua

deps/rust/.dir:
	git clone --depth 1 https://github.com/rust-lang/rust.git deps/rust
	touch deps/rust/.dir

deps/lua/.dir: deps/lua/lua-${LUA_VERSION}/.dir
	touch deps/lua/.dir

deps/lua/lua-${LUA_VERSION}/.dir:
	mkdir -p deps/lua
	cd deps/lua && curl "${LUA_URL}" | tar xz
	touch deps/lua/lua-${LUA_VERSION}/.dir

.PHONY: all-deps clean-deps clean-dep-sources
