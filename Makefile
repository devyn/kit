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

KERNEL_CCFLAGS=-std=c99 -pedantic -Wall -Wextra -Werror -ffreestanding -fno-exceptions -fomit-frame-pointer -mcmodel=large -O2 -mno-red-zone -mno-mmx -mno-sse -mno-sse2 -mno-sse3 -mno-3dnow
KERNEL_LDFLAGS=-ffreestanding -O2 -nostdlib -Wl,-z,max-page-size=0x1000
KERNEL_ASFLAGS=

KERNEL_OBJECTS=$(addprefix build/kernel/, boot32.o boot64.o kernel.o terminal.o memory.o interrupt.o interrupt_isr_stub.o interrupt_8259pic.o test.o)

all-kernel: build/kernel/kernel.bin

build/kernel/kernel.bin: ${KERNEL_OBJECTS} kernel/scripts/link.ld build/kernel/.dir
	${CC} ${LDFLAGS} ${KERNEL_LDFLAGS} -T kernel/scripts/link.ld -o build/kernel/kernel.bin ${KERNEL_OBJECTS}

build/kernel/boot32.o: kernel/boot32.S build/kernel/.dir
	${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} kernel/boot32.S -o build/kernel/boot32.o

build/kernel/boot64.o: kernel/boot64.S build/kernel/.dir
	${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} kernel/boot64.S -o build/kernel/boot64.o

build/kernel/interrupt_isr_stub.o: kernel/interrupt_isr_stub.S build/kernel/.dir
	${AS} ${ASFLAGS} ${KERNEL_ASFLAGS} kernel/interrupt_isr_stub.S -o build/kernel/interrupt_isr_stub.o

build/kernel/%.o: kernel/%.c build/kernel/.dir
	${CC} ${CCFLAGS} ${KERNEL_CCFLAGS} -I kernel/include -c $< -o $@

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
	qemu-system-x86_64 -cdrom build/kit.iso -boot d ${QEMUFLAGS}

.PHONY: all all-kernel all-iso clean clean-kernel clean-iso run-qemu
