################################################################################
#
# kit/system/kforth/kforth.mk
# - build rules for kFORTH
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

KFORTH_OBJECTS := $(patsubst %.c,build/%.o,$(wildcard system/kforth/*.c))

all-kforth: build/system/bin/kforth

clean-kforth:
	rm -rf build/system/kforth
	rm -f build/system/bin/kforth

.PHONY: all-kforth clean-kforth

build/system/kforth/.dir: build/system/.dir
	mkdir -p build/system/kforth
	touch build/system/kforth/.dir

build/system/bin/kforth: ${KFORTH_OBJECTS} ${LIBC} \
		build/system/bin/.dir
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${SYSTEM_LDFLAGS} ${KFORTH_OBJECTS} ${LIBC} \
		-o $@

build/system/kforth/%.o: system/kforth/%.c build/system/kforth/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${SYSTEM_CFLAGS} -I system/kforth/include -c $< -o $@
