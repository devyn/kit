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

.section .bootstrap_gdt64

gdt64:
gdt64_null:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0             # Access
  .byte 0             # low nibble: Limit (high)
  .byte 0             # Base (high)

gdt64_code:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b10011000    # Access: Present, <>, ring 0, executable
  .byte 0b00100000    # high nibble: Long mode bit set, low nibble: Limit (high)
  .byte 0             # Base (high)

gdt64_data:
  .word 0             # Limit (low)
  .word 0             # Base (low)
  .byte 0             # Base (middle)
  .byte 0b10010000    # Access: Present, <>, ring 0
  .byte 0b00000000    # low nibble: Limit (high)
  .byte 0             # Base (high)

gdt64_pointer:
  .word . - gdt64 - 1 # Limit
  .quad gdt64         # Base

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

.initializePaging:
  # Set up (temporary) paging the way x86_64 long mode likes it.
  # The result of this is a page table in which the first two megabytes of physical
  # memory are identity mapped (0-2MB physical -> 0-2MB virtual)

  # Disable paging (though it probably isn't enabled at the moment anyway)

  movl %cr0, %eax              # Load control register 0 for modification in EAX
  andl $((1 << 31) - 1), %eax  # Clear bit 31 (PG), which determines paging state
  movl %eax, %cr0              # Save control register 0

  # Clear some memory at 0x1000 for our temporary page table

  movl $0x1000, %edi           # Destination: 0x1000
  movl %edi,    %cr3           # Set control register 3 (PML4T location) to destination
  xorl %eax,    %eax           # Set output to zero
  movl $4096,   %ecx           # Set count to 4096 (16 KB)
  rep  stosl                   # Execute clear

  movl %cr3,    %edi           # Set destination to control register 3 again

  # Set up PML4T, PDPT, and PDT, eventually pointing to 0x4000 as the PT location
  # Note the 3 at the end of each address - we need to set the least two bits which
  # indicate Present (bit 0) and Read/Write (bit 1) respectively

  movl $0x2003, (%edi)         # Set PML4T[0] to 0x2003 (0x2000 + P,R)
  addl $0x1000,  %edi          # Advance destination 1 KB to 0x2000
  movl $0x3003, (%edi)         # Set PDPT[0] to 0x3003 (0x3000 + P,R)
  addl $0x1000,  %edi          # Advance destination 1 KB to 0x3000
  movl $0x4003, (%edi)         # Set PDT[0] to 0x4003 (0x4000 + P,R)
  addl $0x1000,  %edi          # Advance destination 1 KB to 0x4000

  # Set up PT to identity map the first 2 MB of memory as promised.

  movl $0x00000003,  %eax      # Initial value is address 0 with the Present and Read/Write bits set
  movl $512,         %ecx      # 512 entries * 1024 bytes mapped per entry = 2 megabytes

.LsetPageTableEntries:
  movl %eax,        (%edi)     # Set current table entry to value (normally quad)
  addl $0x1000,      %eax      # Advance value (each entry maps 1 KB)
  addl $8,           %edi      # Advance entry destination (8 bytes per quad)
  loop .LsetPageTableEntries   # Loop until ECX runs out

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

  lgdt gdt64_pointer           # Load Global Descriptor Table.
  jmp  $0x10, $_bootstrap64    # Far jump to _bootstrap64 with GDT64 Code segment.

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
