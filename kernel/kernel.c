// Based on OSDev Bare Bones tutorial
// http://wiki.osdev.org/Bare_Bones
 
#include "terminal.h"

/* Check if the compiler thinks if we are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif
 
#if defined(__cplusplus)
extern "C" /* Use C linkage for kernel_main. */
#endif
void kernel_main()
{
	terminal_initialize();
	terminal_writestring("Hello, kernel World!\nNewline test!");
}
