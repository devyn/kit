.section .rodata

# FPU init word (div by zero + invalid operand cause exceptions)
.global FPU_INIT
FPU_INIT: .word 0x37a

# SSE init dword (same config)
.global SSE_INIT
SSE_INIT: .long 0x1d00
