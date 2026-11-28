/* QEMU virt: RAM da 0x40000000, kernel a 0x40080000. */
ENTRY(_start)

SECTIONS {
    . = 0x40080000;
    .text : { KEEP(*(.text.boot)) *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    .bss : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*)
        __bss_end = .;
    }
    .stack (NOLOAD) : ALIGN(16) {
        . += 0x4000;
    }
    __stack_top = .;
    /DISCARD/ : { *(.eh_frame*) }
}
