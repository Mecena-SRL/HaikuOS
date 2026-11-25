ENTRY(_start)

SECTIONS
{
    . = 0x100000;

    .text : { KEEP(*(.text.boot)) *(.text .text.*) }
    .rodata : ALIGN(16) { *(.rodata .rodata.*) }
    .data : ALIGN(16) { *(.data .data.*) }

    .bss (NOLOAD) : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*) *(COMMON)
        __bss_end = .;
    }

    .stack (NOLOAD) : ALIGN(16) {
        . += 64K;
        __stack_top = .;
    }

    /DISCARD/ : { *(.eh_frame*) *(.note*) *(.comment) }
}
