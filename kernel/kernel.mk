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
              -fno-exceptions -fno-omit-frame-pointer -mcmodel=kernel \
              -march=core2 -mtune=generic -mno-red-zone -mno-mmx -mno-sse3 \
              -mno-ssse3 -mno-3dnow
KERNEL_LDFLAGS=-O1 -nostdlib -z max-page-size=0x1000
KERNEL_ASFLAGS=-march=generic64
KERNEL_RUSTFLAGS=--target x86_64-unknown-linux-gnu \
								 -C debuginfo=2 -C target-cpu=generic \
								 -C target-feature=-sse3,-ssse3,-3dnow \
								 -C no-redzone -C code-model=kernel \
								 -C relocation-model=static -C opt-level=2 -Z no-landing-pads \
								 -L build/deps/kernel -L build/deps/kernel/nolink --sysroot ""

ifeq ($(CC),clang)
	KERNEL_CFLAGS+=-target x86_64-pc-none-elf
endif

KERNEL_RUST_SRC:=$(shell find kernel/ -type f -name '*.rs')

KERNEL_OBJECTS:=$(addprefix build/,$(patsubst %.c,%.o,$(wildcard kernel/*.c)))
KERNEL_OBJECTS+=$(addprefix build/,$(patsubst %.S,%.o,$(wildcard kernel/*.S)))
KERNEL_OBJECTS+=build/kernel/kernel.o

all-kernel: build/kernel.elf

doc-kernel: build/doc/kernel/.dir

clean-kernel:
	rm -rf build/kernel
	rm -f build/kernel.elf

.PHONY: all-kernel doc-kernel clean-kernel

build/kernel/.dir: build/.dir
	mkdir -p build/kernel
	touch build/kernel/.dir

build/doc/kernel/.dir: build/doc/.dir ${KERNEL_OBJECTS}
	rustdoc --cfg doc -w html -o build/doc kernel/kernel.rs
	touch build/doc/kernel/.dir

build/kernel.elf: ${KERNEL_OBJECTS} kernel/scripts/link.ld
	@${ECHO_LD} $@
	@${LD} ${LDFLAGS} ${KERNEL_LDFLAGS} -T kernel/scripts/link.ld -o $@ \
		${KERNEL_OBJECTS} --start-group ${KERNEL_RLIB_DEPS} --end-group

build/kernel/%.o: kernel/%.S build/kernel/.dir
	@${ECHO_AS} $@
	@${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} $< -o $@

build/kernel/%.o: kernel/%.c build/kernel/.dir
	@${ECHO_CC} $@
	@${CC} ${CFLAGS} ${KERNEL_CFLAGS} -I kernel/include -c $< -o $@

build/kernel/kernel.o: kernel/kernel.rs ${KERNEL_RUST_SRC} ${KERNEL_RLIB_DEPS} build/kernel/.dir
	@${ECHO_RUSTC} $@
	@${RUSTC} ${RUSTFLAGS} ${KERNEL_RUSTFLAGS} --emit obj $< -o $@
