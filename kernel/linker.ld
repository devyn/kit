/* Based on OSDev Bare Bones tutorial
   http://wiki.osdev.org/Bare_Bones */

/* The bootloader will look at this image and start execution at the symbol
   designated as the entry point. */
ENTRY(_bootstrap)

/* Tell where the various sections of the object files will be put in the final
   kernel image. */
SECTIONS
{
  /* Begin putting sections at 1 MiB, a conventional place for kernels to be
     loaded at by the bootloader. */
  . = 1M;
  _bootstrap_begin = .;

  /* This is our bootstrap program that runs in protected mode, and includes the
   * multiboot header. */
  .bootstrap BLOCK(4K) :
  {
    build/kernel/boot32.o (.multiboot)
    build/kernel/boot32.o (.text)
  }
  . = ALIGN(4K);

  .bootstrap_data :
  {
    build/kernel/boot32.o (.bootstrap_stack)
    build/kernel/boot32.o (.rodata)
    build/kernel/boot32.o (.data)
  }
  . = ALIGN(4K);

  _kernel_begin = .;

  .text :
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .text)
  }
  . = ALIGN(4K);

  /* Read-only data. */
  .rodata :
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .rodata)
  }
  . = ALIGN(4K);

  /* Read-write data (initialized) */
  .data :
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .data)
  }
  . = ALIGN(4K);

  /* Read-write data (uninitialized) */
  .bss :
  {
    *(COMMON)
    *(.bss)
  }

  /* The compiler may produce other sections, by default it will put them in
     a segment with the same name. Simply add stuff here as needed. */

  /* STABS debugging */
  /*
  .stab          BLOCK(4K) : { *(.stab) }
  .stabstr                 : { *(.stabstr) }
  .stab.excl               : { *(.stab.excl) }
  .stab.exclstr            : { *(.stab.exclstr) }
  .stab.index              : { *(.stab.index) }
  .stab.indexstr           : { *(.stab.indexstr) }
  .comment                 : { *(.comment) }
  */

  _kernel_end = .;

  /* only enable this when not debugging */
  /DISCARD/ :
  {
    *(.comment)
  }
}
