################################################################################
#
# kit/kernel/memory.S
# - physical memory management and kernel heap (low-level helpers)
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015-2021, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .bss

# 128 KiB of initial heap to be used before proper heap initialization
.comm MEMORY_INITIAL_HEAP, 131072, 4096
