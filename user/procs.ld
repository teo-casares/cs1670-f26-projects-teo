ENTRY(main)

PHDRS
{
  text    PT_LOAD    FLAGS(5); /* R-X */
  data    PT_LOAD    FLAGS(6); /* RW- */
  dynamic PT_DYNAMIC FLAGS(6); /* RW- */
}

SECTIONS
{
  . = 0x00000000;

  .text : {
    *(.text)
    *(.text.*)
  } :text

  . = ALIGN(16);
  .rodata : {
    *(.rodata)
    *(.rodata.*)
  } :text

  . = ALIGN(16);
  .rela.dyn : {
    *(.rela.dyn)
  } :text

  . = ALIGN(16);
  .dynamic : {
    *(.dynamic)
  } :data :dynamic

  .data.rel.ro : {
    *(.data.rel.ro)
    *(.data.rel.ro.*)
  } :data

  .got : {
    *(.got)
    *(.got.*)
  } :data

  .data : {
    *(.data)
    *(.data.*)
  } :data

  .bss : {
    *(.bss)
    *(.bss.*)
    *(COMMON)
  } :data
}
