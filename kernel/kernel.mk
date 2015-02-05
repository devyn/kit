################################################################################
#
# kit/kernel/kernel.mk
# - build rules for the kit kernel
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

KERNEL_CFLAGS=-O3 -g -std=c99 -pedantic -Wall -Wextra -Werror -ffreestanding \
              -fno-exceptions -fomit-frame-pointer -mcmodel=kernel \
              -mno-red-zone -mtune=core2 -mno-mmx -mno-sse3 -mno-ssse3 \
              -mno-3dnow
KERNEL_LDFLAGS=-O -nostdlib -z max-page-size=0x1000
KERNEL_ASFLAGS=-march=generic64

ifeq ($(CC),clang)
	KERNEL_CFLAGS+=-target x86_64-pc-none-elf
endif

KERNEL_OBJECTS:=$(addprefix build/, $(patsubst %.c,%.o,$(wildcard kernel/*.c)))
KERNEL_OBJECTS+=$(addprefix build/, $(patsubst %.S,%.o,$(wildcard kernel/*.S)))

all-kernel: build/kernel/kernel.bin

clean-kernel:
	rm -rf build/kernel

.PHONY: all-kernel clean-kernel

build/kernel/.dir: build/.dir
	mkdir -p build/kernel
	touch build/kernel/.dir

build/kernel/kernel.bin: ${KERNEL_OBJECTS} kernel/scripts/link.ld build/kernel/.dir
	@${ECHO_LD} build/kernel/kernel.bin
	@${LD} ${LDFLAGS} ${KERNEL_LDFLAGS} -T kernel/scripts/link.ld -o build/kernel/kernel.bin ${KERNEL_OBJECTS}

build/kernel/%.o: kernel/%.S build/kernel/.dir
	@${ECHO_AS} $@
	@${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} $< -o $@

build/kernel/%.o: kernel/%.c build/kernel/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${KERNEL_CFLAGS} -I kernel/include -c $< -o $@
