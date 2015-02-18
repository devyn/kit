################################################################################
#
# kit/system/system.mk
# - build rules for the system/userland
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

SYSTEM_CFLAGS=-O2 -g -std=c99 -pedantic -Wall -Wextra -Werror \
              -march=core2 -mtune=generic -mno-mmx -mno-sse3 -mno-ssse3 \
              -mno-3dnow -mno-sse -mno-sse2 -nostdlibinc
SYSTEM_LDFLAGS=-O1 -nostdlib
SYSTEM_ASFLAGS=-march=generic64

ifeq ($(CC),clang)
	SYSTEM_CFLAGS+=-target x86_64-pc-none-elf
endif

all-system: build/system.kit

clean-system:
	rm -f build/system.kit
	rm -rf build/system

.PHONY: all-system clean-system

build/system/.dir: build/.dir
	mkdir -p build/system
	touch build/system/.dir

build/system/bin/.dir: build/system/.dir
	mkdir -p build/system/bin
	touch build/system/bin/.dir

build/system/hello.txt: system/hello.txt build/system/.dir
	cp $< $@

include system/libc/libc.mk
include system/util/util.mk
include system/shell/shell.mk

build/system.kit: build/system/hello.txt ${SYSTEM_UTILS} build/system/bin/shell
	ruby resources/build-util/kit-archive.rb build/system \
		$(patsubst build/system/%,%,$^) \
		> $@
