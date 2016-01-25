################################################################################
#
# kit/system/kitforth/kitforth.mk
# - build rules for kitFORTH
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

KFORTH_OBJECTS := $(patsubst %.c,build/%.o,$(wildcard system/kitforth/*.c))
KFORTH_OBJECTS+=$(patsubst %.S,build/%.o,$(wildcard system/kitforth/*.S))

all-kitforth: build/system/bin/kitforth

clean-kitforth:
	rm -rf build/system/kitforth
	rm -f build/system/bin/kitforth

.PHONY: all-kitforth clean-kitforth

build/system/kitforth/.dir: build/system/.dir
	mkdir -p build/system/kitforth
	touch build/system/kitforth/.dir

build/system/bin/kitforth: ${KFORTH_OBJECTS} ${LIBC} \
		build/system/bin/.dir
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${SYSTEM_LDFLAGS} ${KFORTH_OBJECTS} ${LIBC} \
		-o $@

build/system/kitforth/%.o: system/kitforth/%.c build/system/kitforth/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${SYSTEM_CFLAGS} -I system/kitforth/include -c $< -o $@

build/system/kitforth/%.o: system/kitforth/%.S build/system/kitforth/.dir
	@${ECHO_AS} $@
	@${AS} ${ASFLAGS} ${SYSTEM_ASFLAGS} -c $< -o $@
