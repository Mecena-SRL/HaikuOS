ENTRY(_start)

SECTIONS {
    . = 1M;

    .multiboot : { KEEP(*(.multiboot2)) }
    .text : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    /DISCARD/ : { *(.eh_frame) *(.note*) }
}
