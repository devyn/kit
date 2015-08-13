################################################################################
#
# kit/deps/system_deps.mk
# - build rules for all system dependencies
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

build/deps/system/.dir: build/deps/.dir
	mkdir -p build/deps/system
	touch build/deps/system/.dir

build/deps/system/liblua.a: deps/lua/.dir build/deps/system/.dir
	@${ECHO_MAKE} build/deps/system/liblua.a
	@cd deps/lua/lua-${LUA_VERSION}/src && \
		${MAKE} "CC=${CC}" "CFLAGS=${SYSTEM_CFLAGS}" \
			"SYSLDFLAGS=${SYSTEM_LDFLAGS}" liblua.a
	@cp deps/lua/lua-${LUA_VERSION}/src/liblua.a build/deps/system/liblua.a

all-system-deps: build/deps/system/liblua.a

clean-system-deps:
	rm -rf build/deps/system
	[[ ! -d deps/lua/lua-${LUA_VERSION}/src ]] || \
		(cd deps/lua/lua-${LUA_VERSION}/src && make clean)

.PHONY: all-system-deps clean-system-deps
