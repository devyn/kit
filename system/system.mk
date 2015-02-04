################################################################################
#
# kit/system/system.mk
# - build rules for the system/userland
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2013, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

SYSTEM_LDFLAGS=-O -nostdlib
SYSTEM_ASFLAGS=-march=generic64

all-system: build/system.kit

clean-system:
	rm -f build/system.kit
	rm -rf build/system

.PHONY: all-system clean-system

build/system/.dir: build/.dir
	mkdir -p build/system
	touch build/system/.dir

build/system/hello.txt: system/hello.txt build/system/.dir
	cp $< $@

build/system/usertest.bin: system/usertest/usertest.S build/system/.dir
	@${ECHO_AS} build/system/usertest.o
	@${AS} ${ASFLAGS} ${SYSTEM_ASFLAGS} $< -o build/system/usertest.o

	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${SYSTEM_LDFLAGS} build/system/usertest.o -o $@

	rm build/system/usertest.o

build/system.kit: build/system/hello.txt build/system/usertest.bin
	ruby resources/build-util/kit-archive.rb build/system > $@
