################################################################################
#
# kit/system/libc/libc.mk
# - C library build rules
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

LIBC=build/system/libc.o

LIBC_LDFLAGS=

LIBC_OBJECTS:=$(patsubst %.c,build/%.o,$(wildcard system/libc/*.c))
LIBC_OBJECTS+=$(patsubst %.S,build/%.o,$(wildcard system/libc/*.S))

all-libc: build/system/libc.o

clean-libc:
	rm -rf build/system/libc
	rm build/system/libc.o

.PHONY: all-libc clean-libc

build/system/libc/.dir:
	mkdir -p build/system/libc
	touch build/system/libc/.dir

build/system/libc/%.o: system/libc/%.c build/system/libc/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${SYSTEM_CFLAGS} -c $< -o $@

build/system/libc/%.o: system/libc/%.S build/system/libc/.dir
	@${ECHO_AS} $@
	@${AS} ${ASFLAGS} ${SYSTEM_ASFLAGS} -c $< -o $@

${LIBC}: ${LIBC_OBJECTS}
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${LIBC_LDFLAGS} -r $^ -o $@
