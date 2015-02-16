################################################################################
#
# kit/system/shell/shell.mk
# - build rules for the kit shell
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

SHELL_OBJECTS := $(patsubst %.c,build/%.o,$(wildcard system/shell/*.c))

all-shell: build/system/bin/shell

clean-shell:
	rm -rf build/system/shell
	rm -f build/system/bin/shell

.PHONY: all-shell clean-shell

build/system/shell/.dir: build/system/.dir
	mkdir -p build/system/shell
	touch build/system/shell/.dir

build/system/bin/shell: ${SHELL_OBJECTS} ${LIBC} \
		build/system/bin/.dir
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${SYSTEM_LDFLAGS} ${SHELL_OBJECTS} ${LIBC} \
		-o $@

build/system/shell/%.o: system/shell/%.c build/system/shell/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${SYSTEM_CFLAGS} -I system/shell/include -c $< -o $@
