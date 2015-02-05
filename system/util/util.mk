################################################################################
#
# kit/system/util/util.mk
# - build rules for basic system utilities
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

SYSTEM_UTILS:=$(addprefix build/, $(patsubst %.c,%,$(wildcard system/util/*.c)))

all-system-util: ${SYSTEM_UTILS}

.PHONY: all-system-util

build/system/util/.dir:
	mkdir -p build/system/util
	touch build/system/util/.dir

build/system/util/%.o: system/util/%.c build/system/util/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${SYSTEM_CFLAGS} -c $< -o $@

build/system/util/%: build/system/util/%.o build/system/stub.o
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${SYSTEM_LDFLAGS} $^ -o $@
