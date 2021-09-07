################################################################################
#
# kit/Makefile
# - top-level build rules and definitions
#
# vim:ts=2:sw=2:et:tw=80:ft=make
#
# Copyright (C) 2015-2021, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

CC    = clang
AS    = as
CARGO = cargo
LD    = ld

export CPATH=system/libc/include

GRUB_LIB=/usr/lib/grub

ECHO_CC    = echo "[36m    CC [0m"
ECHO_AS    = echo "[36m    AS [0m"
ECHO_RUSTC = echo "[36m RUSTC [0m"
ECHO_LD    = echo "[36m    LD [0m"

all: all-deps all-kernel all-system all-iso

doc: doc-kernel

clean: clean-deps clean-kernel clean-system clean-iso clean-doc

clean-doc:
	rm -rf build/doc

.PHONY: all doc clean clean-doc

build/.dir:
	mkdir -p build
	touch build/.dir

build/doc/.dir: build/.dir
	mkdir -p build/doc
	touch build/doc/.dir

include deps/deps.mk
include kernel/kernel.mk
include system/system.mk

# =ISO Image=

all-iso: build/kit.iso

clean-iso:
	rm -f build/kit.iso
	rm -rf build/isodir

.PHONY: all-iso clean-iso

build/kit.iso: resources/grub.cfg build/kernel.elf build/system.kit
	mkdir -p build/isodir/boot/grub
	mkdir -p build/efidir/EFI/BOOT
	cp resources/grub.cfg build/isodir/boot/grub/grub.cfg
	cp build/kernel.elf build/isodir/boot/kernel.elf
	cp build/system.kit build/isodir/boot/system.kit
	grub-mkimage --format=i386-pc --output=build/core.img -p '/boot/grub' \
		--config=build/isodir/boot/grub/grub.cfg \
    biosdisk iso9660 normal multiboot vga_text at_keyboard
	cat ${GRUB_LIB}/i386-pc/cdboot.img build/core.img \
		> build/isodir/grub.img
	grub-mkimage --format=i386-efi \
		--output=build/efidir/EFI/BOOT/BOOTIA32.EFI \
		-p '/boot/grub' \
		--config=build/isodir/boot/grub/grub.cfg \
		nativedisk iso9660 normal multiboot efi_gop at_keyboard
	grub-mkimage --format=x86_64-efi \
		--output=build/efidir/EFI/BOOT/BOOTX64.EFI \
		-p '/boot/grub' \
		--config=build/isodir/boot/grub/grub.cfg \
		nativedisk iso9660 normal multiboot efi_gop at_keyboard
	rm -f build/isodir/efi.img
	fallocate -l 16M build/isodir/efi.img
	mkfs.vfat build/isodir/efi.img
	mcopy -i build/isodir/efi.img -s build/efidir/* "::"
	xorriso -as mkisofs \
		-R -J -A "Kit" -input-charset "iso8859-1" -R \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		-b grub.img \
		-c boot.cat \
		-eltorito-alt-boot -e efi.img \
		-no-emul-boot -isohybrid-gpt-basdat \
		-efi-boot-part --efi-boot-image \
		-o build/kit.iso \
		build/isodir

# =Testing=

run-qemu: build/kit.iso
	qemu-system-x86_64 -cdrom build/kit.iso -boot d -serial stdio ${QEMUFLAGS}

.PHONY: run-qemu
