/* ROM su cartuccia, dati in IWRAM (copiati dalla ROM), bss in EWRAM. */
ENTRY(_start)

MEMORY {
    ROM   (rx)  : ORIGIN = 0x08000000, LENGTH = 32M
    EWRAM (rwx) : ORIGIN = 0x02000000, LENGTH = 256K
    IWRAM (rwx) : ORIGIN = 0x03000000, LENGTH = 32K
}

SECTIONS {
    .text : {
        KEEP(*(.text.start))
        *(.text .text.*)
        *(.rodata .rodata.*)
        . = ALIGN(4);
    } > ROM

    .data : {
        __sdata = .;
        *(.data .data.*)
        . = ALIGN(4);
        __edata = .;
    } > IWRAM AT> ROM
    __sidata = LOADADDR(.data);

    .bss (NOLOAD) : {
        __sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __ebss = .;
    } > EWRAM

    /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) }
}
