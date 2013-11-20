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
  . = 0x100000;
  _bootstrap_begin = .;

  .bootstrap BLOCK(4K) :
  {
    build/kernel/boot32.o (.multiboot)
    build/kernel/boot32.o (.text)
  }

  .bootstrap_data BLOCK(4K) : ALIGN(4K)
  {
    build/kernel/boot32.o (.bootstrap_gdt64)
    build/kernel/boot32.o (.bootstrap_stack)
    build/kernel/boot32.o (.rodata)
    build/kernel/boot32.o (.data)
  }

  _kernel_begin = .;

  .text BLOCK(4K) : ALIGN(4K)
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .text)
  }

  /* Read-only data. */
  .rodata BLOCK(4K) : ALIGN(4K)
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .rodata)
  }

  /* Read-write data (initialized) */
  .data BLOCK(4K) : ALIGN(4K)
  {
    *(EXCLUDE_FILE(build/kernel/boot32.o) .data)
  }

  /* Read-write data (uninitialized) */
  .bss BLOCK(4K) : ALIGN(4K)
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
