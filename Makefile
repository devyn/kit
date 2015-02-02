CC=clang
AS=as
LD=ld

GRUB_LIB=/usr/lib/grub

all: all-kernel all-system all-iso

clean: clean-kernel clean-system clean-iso

build/.dir:
	mkdir -p build
	touch build/.dir

# =Kernel=

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

build/kernel/kernel.bin: ${KERNEL_OBJECTS} kernel/scripts/link.ld build/kernel/.dir
	@echo -e "\e[36m LD \e[0m" build/kernel/kernel.bin
	@${LD} ${LDFLAGS} ${KERNEL_LDFLAGS} -T kernel/scripts/link.ld -o build/kernel/kernel.bin ${KERNEL_OBJECTS}

build/kernel/%.o: kernel/%.S build/kernel/.dir
	@echo -e "\e[36m AS \e[0m" $@
	@${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} $< -o $@

build/kernel/%.o: kernel/%.c build/kernel/.dir
	@echo -e "\e[36m CC \e[0m" $@
	@${CC} ${CFLAGS} ${KERNEL_CFLAGS} -I kernel/include -c $< -o $@

build/kernel/.dir: build/.dir
	mkdir -p build/kernel
	touch build/kernel/.dir

clean-kernel:
	rm -rf build/kernel

# =System=

all-system: build/system.kit

build/system/hello.txt: system/hello.txt build/system/.dir
	cp $< $@

build/system/usertest.bin: system/usertest/usertest.S build/system/.dir
	@echo -e "\e[36m AS \e[0m" build/system/usertest.o
	@${AS} ${ASFLAGS} $< -o build/system/usertest.o
	@echo -e "\e[36m LD \e[0m" $@
	@${LD} ${LDFLAGS} build/system/usertest.o -o $@
	rm build/system/usertest.o

build/system.kit: build/system/hello.txt build/system/usertest.bin
	ruby resources/build-util/kit-archive.rb build/system > $@

build/system/.dir: build/.dir
	mkdir -p build/system
	touch build/system/.dir

clean-system:
	rm -f build/system.kit
	rm -rf build/system

# =ISO Image=

all-iso: build/kit.iso

build/kit.iso: resources/grub.cfg build/kernel/kernel.bin build/system.kit
	mkdir -p build/isodir/boot/grub
	cp resources/grub.cfg build/isodir/boot/grub/grub.cfg
	cp build/kernel/kernel.bin build/isodir/boot/kernel.bin
	cp build/system.kit build/isodir/boot/system.kit
	grub-mkimage --format=i386-pc --output=build/core.img \
		--config=build/isodir/boot/grub/grub.cfg biosdisk iso9660 normal multiboot
	cat ${GRUB_LIB}/i386-pc/cdboot.img build/core.img > build/isodir/grub.img
	rm build/core.img
	genisoimage -A "Kit" -input-charset "iso8859-1" -R -b grub.img \
		-no-emul-boot -boot-load-size 4 -boot-info-table -o build/kit.iso \
		build/isodir

clean-iso:
	rm -f build/kit.iso
	rm -rf build/isodir

# =Testing=

run-qemu: build/kit.iso
	qemu-system-x86_64 -cdrom build/kit.iso -boot d ${QEMUFLAGS}

.PHONY: all all-kernel all-system all-iso \
        clean clean-kernel clean-system clean-iso \
        run-qemu
