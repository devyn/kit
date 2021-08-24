################################################################################
#
# kit/kernel/boot32.S
# - handles the transition from multiboot entry (32-bit) to long mode
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
# Credit and inspiration for various bits of code in this:
# - http://wiki.osdev.org/Bare_Bones
# - http://wiki.osdev.org/User:Stephanvanschaik/Setting_Up_Long_Mode
# - http://wiki.osdev.org/Creating_a_64-bit_kernel
#
################################################################################

.include "constants.S"

# Declare constants used for creating a multiboot header.
.set ALIGN,    1<<0             # align loaded modules on page boundaries
.set MEMINFO,  1<<1             # provide memory map
.set FLAGS,    ALIGN | MEMINFO  # this is the Multiboot 'flag' field
.set MAGIC,    0x1BADB002       # 'magic number' lets bootloader find the header
.set CHECKSUM, -(MAGIC + FLAGS) # checksum of above, to prove we are multiboot

# Declare a header as in the Multiboot Standard. We put this into a special
# section so we can force the header to be in the start of the final program.
# You don't need to understand all these details as it is just magic values that
# is documented in the multiboot standard. The bootloader will search for this
# magic sequence and recognize us as a multiboot kernel.
.section .multiboot
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM

# Currently the stack pointer register (esp) points at anything and using it may
# cause massive harm. Instead, we'll provide our own stack. We will allocate
# room for a small temporary stack by creating a symbol at the bottom of it,
# then allocating 16384 bytes for it, and finally creating a symbol at the top.
.section .bootstrap_stack
.align 16

stack_buffer:
.skip 1024

.global stack_bottom
stack_bottom:
.skip 15 * 1024
stack_top:

.section .bootstrap_gdt64

.align 4096
.global gdt64
gdt64:

# 0000
gdt64_null:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0             # Access
  .byte 0             # low nibble: Limit (high)
  .byte 0             # Base (high)

# 0008
gdt64_kernel_code:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b10011000    # Access: Present, <>, ring 0, executable
  .byte 0b00100000    # high nibble: Long mode bit set, low nibble: Limit (high)
  .byte 0             # Base (high)

# 0010
gdt64_kernel_data:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b10010010    # Access: Present, <>, ring 0, read/write
  .byte 0b00000000    # low nibble: Limit (high)
  .byte 0             # Base (high)

# 0018
gdt64_user_code_32:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b11111000    # Access: Present, <>, ring 3, executable
  .byte 0b00000000    # high nibble: Long mode clear, low nibble: Limit (high)
  .byte 0             # Base (high)

# 0020
gdt64_user_data:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b11110010    # Access: Present, <>, ring 3, read/write
  .byte 0b00000000    # low nibble: Limit (high)
  .byte 0             # Base (high)

# 0028
gdt64_user_code_64:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b11111000    # Access: Present, <>, ring 3, executable
  .byte 0b00100000    # high nibble: Long mode bit set, low nibble: Limit (high)
  .byte 0             # Base (high)

# To be initialized later.
# 0030
gdt64_tss:
  .word 0             # Limit (low)
  .word 0             # Base (15:0)
  .byte 0             # Base (23:16)
  .byte 0b10001001    # Present, Type: non-busy TSS64, ring 0
  .byte 0b00000000    # high nibble: nothing, low nibble: Limit (high)
  .byte 0             # Base (31:24)
  .long 0             # Base (63:32)
  .long 0             # Reserved

.global gdt64_pointer
gdt64_pointer:
  .word . - gdt64 - 1      # Limit
  .quad gdt64              # Base (will be read as 32-bit the first time)

.global tss64
tss64:
  .long 0             # Reserved
  .quad 0             # RSP0
  .quad 0             # RSP1
  .quad 0             # RSP2
  .quad 0             # Reserved
  .quad 0             # IST1
  .quad 0             # IST2
  .quad 0             # IST3
  .quad 0             # IST4
  .quad 0             # IST5
  .quad 0             # IST6
  .quad 0             # IST7
  .quad 0             # Reserved
  .word 0             # Reserved
  .word tss64_limit   # I/O Map Base Address (default out of TSS limit)

.global tss64_limit
.set tss64_limit, . - tss64 - 1

# For convenience.
.global tss_stack
.set tss_stack, KERNEL_VMA + tss64 + 4

.section .data

# Page tables.
.align 4096
pml4:     .skip 4096
pdpt_lo:  .skip 4096
pd_lo:    .skip 4096
pdpt_hi:  .skip 4096
pd_hi:    .skip 4096

# This is where we will store the multiboot information structure.
.global kernel_multiboot_info
kernel_multiboot_info:
.skip 88 # 88 bytes - see multiboot information format

.section .rodata
.Lerr_noCPUID:    .ascii "E: Your processor does not support the CPUID instruction.\0"
.Lerr_noLongMode: .ascii "E: Your processor does not support x86_64 long mode.\0"
.Lerr_fatal:      .ascii "Kit can not continue to boot. Please restart your computer.\0"

# The linker script specifies _bootstrap as the entry point to the kernel
# and the bootloader will jump to this position once the kernel has been
# loaded. It doesn't make sense to return from this function as the
# bootloader is gone.
.section .text
.code32
.global _bootstrap
.type _bootstrap, @function
_bootstrap:
  # Disable interrupts.
  cli

  # To set up a stack, we simply set the esp register to point to the top of
  # our stack (as it grows downwards).
  movl $stack_top, %esp
  movl $stack_top, %ebp

.preserveMultiboot:
  # At the moment, a pointer to the multiboot information structure is stored
  # in EBX. Let's copy that to the space we made in kernel_multiboot_info for it.

  movl %ebx,                   %esi # *EBX is source
  movl $kernel_multiboot_info, %edi # kernel_multiboot_info is destination
  movl $(88 >> 2),             %ecx # 88 bytes / 4 bytes per long
  rep movsl                         # Do the copy

.checkCPUID:
  # Is CPUID supported?
  # Check by attempting to flip bit 21 (ID) in FLAGS. If it can be flipped,
  # CPUID is supported.

  # Copy FLAGS into A and C and flip bit 21 on A
  pushfl
  popl %eax
  movl %eax, %ecx
  xorl $(1 << 21), %eax

  # Put A back into FLAGS
  pushl %eax
  popfl

  # Now read FLAGS into A again...
  pushfl
  popl %eax

  # ...also restore FLAGS to what it was before in case this is successful...
  pushl %ecx
  popfl

  # ...and test if A and C are identical. They shouldn't be, if CPUID is supported.
  xorl %ecx, %eax
  jz .LnoCPUID

.checkLongMode:
  # Is long mode supported?
  # Use extended CPUID function 0x80000001, and check EDX bit 29 (LM).

  # Check for extended CPUID functions support.

  mov $0x80000000, %eax # Execute CPUID function 0x80000000
  cpuid
  cmp $0x80000001, %eax # If returned EAX is less than 0x80000001
  jb .LnoLongMode       # then extended CPUID is not supported.

  # Check for long mode support.

  mov $0x80000001, %eax # Execute CPUID function 0x80000001
  cpuid
  test $(1 << 29), %edx # Test bit 29 (LM) on EDX
  jz .LnoLongMode       # If not set, long mode is not supported.

.initializePaging:
  # Set up (temporary) paging the way x86_64 long mode likes it.
  # The result of this is a page table in which the first eight megabytes of physical
  # memory are identity mapped (0-8MB virtual -> 0-8MB physical), as well as mapped
  # at 0xffffffff80000000 which is accessible via the kernel code model.

  # Disable paging (though it probably isn't enabled at the moment anyway)

  movl %cr0, %eax              # Load control register 0 for modification in EAX
  andl $((1 << 31) - 1), %eax  # Clear bit 31 (PG), which determines paging state
  movl %eax, %cr0              # Save control register 0

  # Clear some memory at (pml4) for our temporary page table

  movl $pml4,   %edi           # Destination: (pml4)
  movl %edi,    %cr3           # Set control register 3 (PML4T location) to destination
  xorl %eax,    %eax           # Set output to zero
  movl $5120,   %ecx           # Set count to 5120 (x4 = bytes of tables)
  rep  stosl                   # Execute clear

  movl %cr3,    %edi           # Set destination to control register 3 again

  # Set up PML4T and PDPT, eventually pointing to pd_lo as the low PD
  # Note the 3 at the end of each address - we need to set the least two bits which
  # indicate Present (bit 0) and Read/Write (bit 1) respectively

  movl $(pdpt_lo + 0x3), (pml4)          # Set pml4[0] to (pdpt_lo + P,R)
  movl $(pd_lo   + 0x3), (pdpt_lo)       # Set pdpt_lo[0] to (pd_lo + P,R)

  # Also map the first 8 physical megabytes of memory at 0xffffffff80000000,
  # which is the start of kernel space. This is a bit more complicated.

  movl $(pdpt_hi + 0x3), (pml4 + 0xff8)    # Set pml4[511] to (pdpt_hi + P,R)
  movl $(pd_hi   + 0x3), (pdpt_hi + 0xff0) # Set pdpt_hi[510] to (pd_hi + P,R)

  # Set up PDs to identity map 8 megabytes of memory in both low and high
  # memory.

  movl $0x00000083,   %eax      # Initial value is address 0 with Present, Read/Write, Page size
  movl $pd_lo,        %esi      # pd_lo table
  movl $4,            %ecx      # 4 entries * 2 megabytes mapped per entry = 8 megabytes
  movl $pd_hi,        %edi      # pd_hi table
  call .setPageDirectoryEntries

  jmp  .enablePAE

.setPageDirectoryEntries:
  movl %eax,        (%esi)     # Set current table entry on pt_lo to value (normally quad)
  movl %eax,        (%edi)     # Set current table entry on pt_hi to value
  addl $0x200000,    %eax      # Advance value (each entry maps 2 MB)
  addl $8,           %esi      # Advance entry offset (8 bytes per quad)
  addl $8,           %edi
  loop .setPageDirectoryEntries # Loop until ECX runs out
  ret

.enablePAE:
  # Enable Physical Address Extension (PAE). This does not enable paging (yet).

  movl %cr4, %eax              # Load control register 4 for modification
  orl  $(1 << 5), %eax         # Set bit 5 (PAE)
  movl %eax, %cr4              # Save control register 4

.initializeLongMode:
  # Do everything necessary to switch to long mode short of loading the GDT64 and
  # jumping to 64-bit code. After this we will be in 64-bit compatibility mode.

  # Set the long mode (LM) bit in the Extended Feature Enable Register (EFER)

  movl $0xC0000080, %ecx       # Select the EFER MSR at code 0xC0000080
  rdmsr                        # Load EFER into EAX for modification
  orl  $(1 << 8), %eax         # Set bit 8 (LM)
  wrmsr                        # Write EFER

  # Finally, enable paging.
  
  movl %cr0, %eax              # Load control register 0 for modification
  orl  $(1 << 31), %eax        # Set bit 31 (PG)
  movl %eax, %cr0              # Save control register 0

.jumpToBoot64:
  # Load the GDT and jump to our 64-bit bootstrapping code.

  lgdt gdt64_pointer                    # Load Global Descriptor Table.
  jmp  $KERNEL_CODE_SEL, $_bootstrap64  # Jump to _bootstrap64 with kernel code selector.

.Lhalt:
  # Halt the system if something went wrong.
  cli
  hlt
  jmp .Lhalt

.LnoCPUID:
  # Print "CPUID is not supported" message and halt
  movl $.Lerr_noCPUID, %eax
  call .termfatal
  jmp .Lhalt

.LnoLongMode:
  # Print "long mode is not supported" message and halt
  movl $.Lerr_noLongMode, %eax
  call .termfatal
  jmp .Lhalt

# Some minimal terminal functions

.termclear:
  pushal

  movl $0xB8000, %edi # video memory
  movl $0xf20,   %eax # white-on-black spaces
  movl $2000,    %ecx # 80 * 25

  cld
  rep stosw

  popal
  ret

.termmsg:
  pushal

  # EAX: pointer to C-style string
  # EDX: offset to print to

  movl $0xB8000,          %edi       # Load video memory location into EDI
  leal (%edi,%edx,2),     %edi       # Add offset to EDI
  xorl %ecx, %ecx                    # Reset counter

.LtermmsgLoopStr:
  movb (%eax,%ecx), %bl              # Read from string
  test %bl, %bl
  jz   .LtermmsgLoopEnd              # If char is zero, end

  movb %bl,        (%edi,%ecx,2)     # Move into character byte
  incl %ecx                          # Increment counter
  jmp  .LtermmsgLoopStr

.LtermmsgLoopEnd:
  popal
  ret

.termfatal:
  pushal

  # EAX: pointer to C-style string

  call .termclear
  popal

  pushal
  movl $0, %edx
  call .termmsg             # Print caller-supplied message

  movl $.Lerr_fatal, %eax
  movl $80,          %edx
  call .termmsg             # Print fatal message

  popal
  ret

# Set the size of the _bootstrap symbol to the current location '.' minus its start.
# This is useful when debugging or when you implement call tracing.
.size _bootstrap, . - _bootstrap
