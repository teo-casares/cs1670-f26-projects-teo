    .section ".text.boot"
    .globl _start

_start:
    mrs x0, MPIDR_EL1 // copy MPIDR_EL1 into the x0 register (move system register to general purpose register)
    and x0, x0, #0xFF // bitwise and, get rid of all but the bottom 8 bits (least significant)
    cbz x0, boot_setup // core 0 jump to boot setup label
parking_lot: // infinite boot loop for cores that arent core 0
    b parking_lot

boot_setup:
    adr x0, bss_begin
    adr x1, bss_end
    sub x1, x1, x0
    bl memzero
    ldr x0, =INITIAL_KERNEL_STACK
    mov sp, x0
    bl kernel_main
    b parking_lot
