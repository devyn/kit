TOOLPATH=tools/bin
CC=${TOOLPATH}/i586-elf-gcc
AS=${TOOLPATH}/i586-elf-as

all: all-kernel all-iso

clean: clean-kernel clean-iso

build/.dir:
	mkdir -p build
	touch build/.dir

# =Kernel=

KERNEL_CCFLAGS=-std=gnu99 -ffreestanding -O2 -Wall -Wextra
KERNEL_LDFLAGS=-ffreestanding -O2 -nostdlib -lgcc

KERNEL_OBJECTS=build/kernel/boot.o build/kernel/kernel.o

all-kernel: build/kernel/kernel.bin

build/kernel/kernel.bin: ${KERNEL_OBJECTS} kernel/linker.ld build/kernel/.dir
	${CC} -T kernel/linker.ld -o build/kernel/kernel.bin ${KERNEL_OBJECTS} ${LDFLAGS} ${KERNEL_LDFLAGS}

build/kernel/boot.o: build/kernel/.dir
	${AS} kernel/boot.s -o build/kernel/boot.o ${ASFLAGS} ${KERNEL_ASFLAGS}

build/kernel/kernel.o: build/kernel/.dir
	${CC} -c kernel/kernel.c -o build/kernel/kernel.o ${CCFLAGS} ${KERNEL_CCFLAGS}

build/kernel/.dir: build/.dir
	mkdir -p build/kernel
	touch build/kernel/.dir

clean-kernel:
	rm -f ${KERNEL_OBJECTS}
	rm -f build/kernel/kernel.bin

# =ISO Image=

all-iso: build/kit.iso

build/kit.iso: resources/grub.cfg build/kernel/kernel.bin build/.dir
	mkdir -p build/isodir/boot/grub
	cp resources/grub.cfg build/isodir/boot/grub/grub.cfg
	cp build/kernel/kernel.bin build/isodir/boot/kernel.bin
	grub-mkrescue -o build/kit.iso build/isodir

clean-iso:
	rm -f build/kit.iso
	rm -rf build/isodir

# =Testing=

run-qemu: build/kit.iso
	qemu-system-x86_64 -cdrom build/kit.iso -boot d

.PHONY: all all-kernel all-iso clean clean-kernel clean-iso run-qemu
