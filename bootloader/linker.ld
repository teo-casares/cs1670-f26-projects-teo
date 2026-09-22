/* The GPU firmware loads the armstub at 0x0. */

ENTRY(_start)

SECTIONS
{
    . = 0x0;
    .text : { KEEP(*(.text.armstub)) *(.text) *(.text.*) }
    .rodata : { *(.rodata) *(.rodata.*) }
    .data : { *(.data) *(.data.*) }
    .bss : { *(.bss) *(.bss.*) *(COMMON) }

    /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.comment) }
}
