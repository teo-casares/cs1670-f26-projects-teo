/* This is a "linker script". The linker is a program that combines multiple
 * files that contain machine code ("object files") into a single file that can
 * be loaded into memory and executed.
 *
 * The linker script determines how the executable file is structured and where
 * in memory the different parts of the executable file are loaded. The linker
 * script is written in a special language that is understood by the linker.
 */

ENTRY(_start)
EXTERN(pi_main)

SECTIONS
{
    . = 0x80000;                    /* Load the kernel at 0x8'0000 */
    /* Sections, names start with a "." */
    .text.boot : { KEEP(*(.text.boot)) }  /* Bootup assembly code */
    .text : { *(.text) *(.text.*) }       /* Kernel code */
    .rodata : { *(.rodata) *(.rodata.*) } /* Read-only variables, literal strings */
    .data.rel.ro : { *(.data.rel.ro) *(.data.rel.ro.*) }
    .data : { *(.data) *(.data.*) }       /* Initialized global variables */
    .got : { *(.got) *(.got.*) }
    . = ALIGN(0x8);     /* Align the next section to an 8-byte boundary */
    bss_begin = .;      /* Creates an assembly symbol "bss_begin" that points
                           to the beginning of the .bss section. */
    .bss : {
        *(.bss) *(.bss.*)               /* Uninitialized global variables */
        *(COMMON)
        . = ALIGN(0x8);                 /* Keep the end of .bss 8-byte aligned
                                           so memzero covers all of it. */
    }
    bss_end = .;        /* Assembly symbol "bss_end" that points to the end of
                           the .bss section. */
    kernel_end = .;     /* Assembly symbol to mark the end of kernel code in
                           memory. Useful to refer to memory after. */
    . = ALIGN(4096);    /* Align the next section to a 4KB (page) boundary */
    /* What follows is a special section that contains (ELF-encoded)
     * executables for user programs. We need this section to ensure programs
     * are loaded into memory until we have disk I/O and a file system. */
    elf_executables_begin = .;  /* Assembly symbol */
    .elf_executables : { KEEP(*(.elf_executables)) }  /* User programs */
    elf_executables_end = .;    /* Assembly symbol */

    /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.comment) }
}

ASSERT(DEFINED(_start), "Quest 1: implement _start in kernel/boot.S")
