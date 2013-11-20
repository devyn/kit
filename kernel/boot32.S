# Based on OSDev Bare Bones tutorial
# http://wiki.osdev.org/Bare_Bones

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
stack_bottom:
.skip 16384 # 16 KiB
stack_top:

.section .data
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
  pushf
  popl %eax
  movl %eax, %ecx
  xorl $(1 << 21), %eax

  # Put A back into FLAGS
  pushl %eax
  popf

  # Now read FLAGS into A again...
  pushf
  popl %eax

  # ...also restore FLAGS to what it was before in case this is successful...
  pushl %ecx
  popf

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
  test $(1 << 29), %edx # Test bit 21 (LM) on EDX
  jz .LnoLongMode       # If not set, long mode is not supported.

.runKernel:
  # We are now ready to actually execute C code. We cannot embed that in an
  # assembly file, so we'll create a kernel.c file in a moment. In that file,
  # we'll create a C entry point called kernel_main and call it here.
  #call kernel_main

  # Halt the system.
.Lhalt:
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
  pusha

  movl $0xB8000, %edi # video memory
  movl $0xf20,   %eax # white-on-black spaces
  movl $2000,    %ecx # 80 * 25

  cld
  rep stosw

  popa
  ret

.termmsg:
  pusha

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
  popa
  ret

.termfatal:
  pusha

  # EAX: pointer to C-style string

  call .termclear
  popa

  pusha
  movl $0, %edx
  call .termmsg             # Print caller-supplied message

  movl $.Lerr_fatal, %eax
  movl $80,          %edx
  call .termmsg             # Print fatal message

  popa
  ret

# Set the size of the _bootstrap symbol to the current location '.' minus its start.
# This is useful when debugging or when you implement call tracing.
.size _bootstrap, . - _bootstrap
