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
KERNEL_VMA = 0xffffffff80000000; /* Virtual (higher half) address */

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

  _bootstrap_data_begin = .;

  .bootstrap_data :
  {
    build/kernel/boot32.o (.bootstrap_gdt64)
    build/kernel/boot32.o (.bootstrap_stack)
    build/kernel/boot32.o (.rodata)
    build/kernel/boot64.o (.rodata)
    build/kernel/boot32.o (.data)
    . = ALIGN(4K);
  }

  _bootstrap_end = .;

  /* 64-bit kernel in the higher half of the 48-bit addressing scheme */
  .             += KERNEL_VMA;
  _kernel_begin  = .;

  /* Kernel executable code. */
  .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VMA)
  {
    _kernel_text_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) .text*)
    _kernel_text_end = ALIGN(4K);
  }

  /* Read-only data. */
  .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VMA)
  {
    _kernel_rodata_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) .rodata*)
    _kernel_rodata_end = ALIGN(4K);
  }

  /* Read-write data (initialized) */
  .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VMA)
  {
    _kernel_data_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) .data*)
    _kernel_data_end = ALIGN(4K);
  }

  /* Global offset table */
  .got ALIGN(4K) : AT(ADDR(.got) - KERNEL_VMA)
  {
    _kernel_got_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) .got*)
    /* TODO: figure out why bogus data necessary to make non-zero offset */
    . = . + 8;
    _kernel_got_end = ALIGN(4K);
  }

  /* Global offset table - PLT (executable) */
  .got.plt ALIGN(4K) : AT(ADDR(.got.plt) - KERNEL_VMA)
  {
    _kernel_got_plt_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) .got.plt*)
    /* TODO: figure out why bogus data necessary to make non-zero offset */
    . = . + 8;
    _kernel_got_plt_end = ALIGN(4K);
  }

  /* Read-write data (uninitialized) */
  .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VMA)
  {
    _kernel_bss_begin = .;
    *(EXCLUDE_FILE(build/kernel/boot*.o) COMMON)
    *(EXCLUDE_FILE(build/kernel/boot*.o) .bss*)
    _kernel_bss_end = ALIGN(4K);
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
