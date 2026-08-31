ENTRY(_start)

SECTIONS
{
    . = 0x0;
    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    .data : {
        *(.data .data.*)
    }
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
}
