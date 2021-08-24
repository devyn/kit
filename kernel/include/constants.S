# Virtual memory offset
.set KERNEL_VMA, 0xffffffff80000000

# FS.base MSR (future thread local storage?)
.set IA32_FS_BASE, 0xc0000100

# GS.base MSR (cpu local storage)
.set IA32_GS_BASE, 0xc0000101
.set IA32_KERNEL_GS_BASE, 0xc0000102

# HwState offsets
.set HWSTATE_KSTACK_POINTER, 0x08
.set HWSTATE_REGISTERS,      0x10

# GDT-related constants
.set KERNEL_CODE_SEL, 0x08
.set KERNEL_DATA_SEL, 0x10
.set USER_CD32_SEL, 0x1b
.set USER_DATA_SEL, 0x23
.set USER_CD64_SEL, 0x2b
.set KERNEL_TSS_SEL,  0x30

# CpuLocalData offsets
.set CPULOCAL_SCRATCH, 0x00
.set CPULOCAL_HWSTATE, 0x20

# HwState offsets
.set HWSTATE_KSTACK_BASE,    0x00
.set HWSTATE_KSTACK_POINTER, 0x08
.set HWSTATE_REGISTERS,      0x10
