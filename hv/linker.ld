/* Freestanding hv — Multiboot1 ADDR load (raw .bin for QEMU). */
ENTRY(_start)

SECTIONS
{
    . = 1M;
    __load_start = .;

    .multiboot : {
        KEEP(*(.multiboot))
    }

    .text : {
        *(.text.boot)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    __load_end = .;

    .bootbss (NOLOAD) : ALIGN(4096) {
        *(.boot.bss)
    }

    .bss (NOLOAD) : {
        *(.bss .bss.*)
        *(COMMON)
    }

    __bss_end = .;

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
    }
}
