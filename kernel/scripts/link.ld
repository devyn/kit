/*******************************************************************************
 *
 * link.ld - kernel link script, for multiboot
 *
 * vim:ts=2:sw=2:et:tw=80:ft=ld
 *
 * Based on OSDev Bare Bones tutorial
 * http://wiki.osdev.org/Bare_Bones
 *
 ******************************************************************************/

/* The bootloader will look at this image and start execution at the symbol
   designated as the entry point. */
ENTRY(_bootstrap)

KERNEL_LMA = 0x100000;           /* Physical address */
KERNEL_VMA = 0xffff800000000000; /* Virtual (higher half) address */

/* Tell where the various sections of the object files will be put in the final
   kernel image. */
SECTIONS
{
  /* 32-bit and 64-bit bootstrap routines at 1 MB */
  .                = KERNEL_LMA;
  _bootstrap_begin = .;

  .bootstrap :
  {
    build/kernel/boot32.o (.multiboot)
    build/kernel/boot32.o (.text)
    . = ALIGN(4K);
  }

  .bootstrap64 :
  {
    build/kernel/boot64.o (.text)
    . = ALIGN(4K);
  }

  .bootstrap_data :
  {
    build/kernel/boot32.o (.bootstrap_gdt64)
    build/kernel/boot32.o (.bootstrap_stack)
    build/kernel/boot32.o (.rodata)
    build/kernel/boot64.o (.rodata)
    build/kernel/boot32.o (.data)
    . = ALIGN(4K);
  }

  /* 64-bit kernel in the higher half of the 48-bit addressing scheme */
  .             += KERNEL_VMA;
  _kernel_begin  = .;

  /* Kernel executable code. */
  .text : AT(ADDR(.text) - KERNEL_VMA)
  {
    *(EXCLUDE_FILE(build/kernel/boot*.o) .text)
    . = ALIGN(4K);
  }

  /* Read-only data. */
  .rodata : AT(ADDR(.rodata) - KERNEL_VMA)
  {
    *(EXCLUDE_FILE(build/kernel/boot*.o) .rodata*)
    . = ALIGN(4K);
  }

  /* Read-write data (initialized) */
  .data : AT(ADDR(.data) - KERNEL_VMA)
  {
    *(EXCLUDE_FILE(build/kernel/boot*.o) .data)
    . = ALIGN(4K);
  }

  /* Read-write data (uninitialized) */
  .bss : AT(ADDR(.bss) - KERNEL_VMA)
  {
    *(EXCLUDE_FILE(build/kernel/boot*.o) COMMON)
    *(EXCLUDE_FILE(build/kernel/boot*.o) .bss)
    . = ALIGN(4K);
  }

  /* The compiler may produce other sections, by default it will put them in
     a segment with the same name. Simply add stuff here as needed. */

  _kernel_end = .;

  /DISCARD/ :
  {
    *(.comment)
    *(.eh_frame)
  }
}
