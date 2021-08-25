# Virtual memory offset
.set KERNEL_VMA, 0xffffffff80000000

# FS.base MSR (future thread local storage?)
.set IA32_FS_BASE, 0xc0000100

# GS.base MSR (cpu local storage)
.set IA32_GS_BASE, 0xc0000101
.set IA32_KERNEL_GS_BASE, 0xc0000102

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
.set HWSTATE_KERN_REGISTERS, 0x10
.set HWSTATE_USER_REGISTERS, 0x50

# HwState offsets (commonly used)
.set HWSTATE_KERN_RIP, HWSTATE_KERN_REGISTERS+0x00
.set HWSTATE_KERN_RSP, HWSTATE_KERN_REGISTERS+0x08
.set HWSTATE_KERN_RBP, HWSTATE_KERN_REGISTERS+0x10
