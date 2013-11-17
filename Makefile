TOOLPATH=tools/bin
CC=${TOOLPATH}/x86_64-elf-gcc
AS=${TOOLPATH}/x86_64-elf-as

export PATH := ${TOOLPATH}:${PATH}

all: all-kernel all-iso

clean: clean-kernel clean-iso

build/.dir:
	mkdir -p build
	touch build/.dir

# =Kernel=

KERNEL_CCFLAGS=-gstabs -std=c99 -pedantic -Wall -Wextra -Werror -ffreestanding -O2 -m32 -march=i586
KERNEL_LDFLAGS=-gstabs -ffreestanding -O2 -nostdlib -m32 -march=i586 -Wl,-m,elf_i386
KERNEL_ASFLAGS=--gstabs --32

KERNEL_OBJECTS=build/kernel/boot.o build/kernel/kernel.o build/kernel/terminal.o

all-kernel: build/kernel/kernel.bin

build/kernel/kernel.bin: ${KERNEL_OBJECTS} kernel/linker.ld build/kernel/.dir
	${CC} ${LDFLAGS} ${KERNEL_LDFLAGS} -T kernel/linker.ld -o build/kernel/kernel.bin ${KERNEL_OBJECTS}

build/kernel/boot.o: kernel/boot.s build/kernel/.dir
	${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} kernel/boot.s -o build/kernel/boot.o

build/kernel/%.o: kernel/%.c build/kernel/.dir
	${CC} ${CCFLAGS} ${KERNEL_CCFLAGS} -c $< -o $@

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

run-qemu-monitor: build/kernel/kernel.bin
	qemu-system-x86_64 -kernel build/kernel/kernel.bin -monitor stdio

run-qemu-debug: build/kernel/kernel.bin
	qemu-system-x86_64 -kernel build/kernel/kernel.bin -s -S

.PHONY: all all-kernel all-iso clean clean-kernel clean-iso run-qemu run-qemu-monitor run-qemu-debug
