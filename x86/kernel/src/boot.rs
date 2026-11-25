// Punto d'ingresso del kernel: stack, azzeramento BSS, poi kmain().
core::arch::global_asm!(
    ".section .text.boot, \"ax\"",
    ".global _start",
    "_start:",
    "lea rsp, [rip + __stack_top]",
    "lea rdi, [rip + __bss_start]",
    "lea rcx, [rip + __bss_end]",
    "sub rcx, rdi",
    "xor eax, eax",
    "rep stosb",
    "call kmain",
    "2:",
    "hlt",
    "jmp 2b",
);
