#include <stdint.h>
#include <stddef.h>

#include "terminal.h"
#include "memory.h"

#include "test.h"

bool test_run(const char *name, bool (*testcase)())
{
  terminal_setcolor(COLOR_LIGHT_CYAN, COLOR_BLACK);

  terminal_writestring("\n[TEST] ");
  terminal_writestring(name);

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_putchar('\n');

  bool result = (*testcase)();

  if (result)
  {
    terminal_setcolor(COLOR_LIGHT_GREEN, COLOR_BLACK);
    terminal_writestring("[PASS] ");
  }
  else
  {
    terminal_setcolor(COLOR_LIGHT_RED, COLOR_BLACK);
    terminal_writestring("[FAIL] ");
  }

  terminal_writestring(name);

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_putchar('\n');

  return result;
}

#define HEADING(heading)                           \
  terminal_setcolor(COLOR_WHITE, COLOR_BLACK);     \
  terminal_writestring((heading));                 \
  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK)

bool test_memory_c()
{
  HEADING("memory_alloc(512) returns a non-NULL pointer\n");

  char *ptr = memory_alloc(512);

  if (ptr == NULL)
  {
    terminal_writestring("  E: returned NULL");
    return false;
  }
  else
  {
    terminal_writestring("  - returned pointer: 0x");
    terminal_writeuint64((uint64_t) ptr, 16);
    terminal_putchar('\n');
  }

  HEADING("memory_clear() clears memory\n");

  size_t i;

  terminal_writestring("  - writing varied data to allocated memory\n");
  
  for (i = 0; i < 512; i++) ptr[i] = i;

  terminal_writestring("  - invoking memory_clear()\n");
  memory_clear(ptr, 512);

  terminal_writestring("  - verifying that the memory has been cleared\n");

  for (i = 0; i < 512; i++)
  {
    if (ptr[i] != 0)
    {
      terminal_writestring("  E: memory not cleared at byte ");
      terminal_writeuint64((uint64_t) i, 10);

      terminal_writestring("; value is 0x");
      terminal_writeuint64((uint64_t) ptr[i], 16);

      terminal_putchar('\n');
      return false;
    }
  }

  HEADING("memory_alloc_aligned(1, 1024) returns the original pointer + 1024\n");

  char *aligned_ptr = memory_alloc_aligned(1, 1024);

  terminal_writestring("  - returned pointer: 0x");
  terminal_writeuint64((uint64_t) aligned_ptr, 16);
  terminal_putchar('\n');

  if (aligned_ptr != ptr + 1024)
  {
    terminal_writestring("  E: not original pointer + 1024\n");
    return false;
  }

  return true;
}
