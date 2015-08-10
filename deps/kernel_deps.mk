################################################################################
#
# kit/deps/kernel_deps.mk
# - build rules for all kernel dependencies
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

KERNEL_RUST_DEPS = core alloc rustc_unicode collections
KERNEL_RLIB_DEPS = $(addprefix build/deps/kernel/lib,$(addsuffix .rlib,${KERNEL_RUST_DEPS}))

build/deps/kernel/.dir: build/deps/.dir
	mkdir -p build/deps/kernel
	touch build/deps/kernel/.dir

build/deps/kernel/libcore.rlib: deps/rust/.dir build/deps/kernel/.dir
	@${ECHO_RUSTC} $@
	@${RUSTC} ${RUSTFLAGS} ${KERNEL_RUSTFLAGS} \
		--crate-type lib --crate-name core \
		--out-dir build/deps/kernel deps/rust/src/libcore/lib.rs

build/deps/kernel/liballoc.rlib: deps/rust/.dir build/deps/kernel/libcore.rlib \
                                 build/deps/kernel/.dir
	@${ECHO_RUSTC} $@
	@${RUSTC} ${RUSTFLAGS} ${KERNEL_RUSTFLAGS} \
		--crate-type lib --crate-name alloc --cfg feature=\"external_funcs\" \
		--out-dir build/deps/kernel deps/rust/src/liballoc/lib.rs

build/deps/kernel/librustc_unicode.rlib: deps/rust/.dir \
                                         build/deps/kernel/libcore.rlib \
                                         build/deps/kernel/.dir
	@${ECHO_RUSTC} $@
	@${RUSTC} ${RUSTFLAGS} ${KERNEL_RUSTFLAGS} \
		--crate-type lib --crate-name rustc_unicode \
		--out-dir build/deps/kernel deps/rust/src/librustc_unicode/lib.rs

build/deps/kernel/libcollections.rlib: deps/rust/.dir \
																			 build/deps/kernel/libcore.rlib \
																			 build/deps/kernel/liballoc.rlib \
	                                     build/deps/kernel/librustc_unicode.rlib \
																			 build/deps/kernel/.dir
	@${ECHO_RUSTC} $@
	@${RUSTC} ${RUSTFLAGS} ${KERNEL_RUSTFLAGS} \
		--crate-type lib --crate-name collections \
		--out-dir build/deps/kernel deps/rust/src/libcollections/lib.rs

all-kernel-deps: ${KERNEL_RLIB_DEPS}

clean-kernel-deps:
	rm -rf build/deps/kernel

.PHONY: all-kernel-deps clean-kernel-deps
