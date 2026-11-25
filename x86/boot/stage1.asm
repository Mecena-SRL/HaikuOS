; Stage 1: settore MBR (LBA 0). Carica stage 2 (LBA 1..32) a 0x0000:0x7E00.
[bits 16]
[org 0x7C00]

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    mov [boot_drive], dl
    mov si, dap
    mov ah, 0x42
    int 0x13
    jc .err
    mov dl, [boot_drive]
    jmp 0x0000:0x7E00
.err:
    mov si, msg
    call print
    cli
    hlt

print:
    lodsb
    test al, al
    jz .r
    mov ah, 0x0E
    mov bx, 7
    int 0x10
    jmp print
.r:
    ret

msg:        db "Errore disco (stage1)", 0
boot_drive: db 0
dap:
    db 0x10, 0
    dw 32           ; 32 settori = 16 KB di stage 2
    dw 0x7E00       ; offset
    dw 0x0000       ; segmento
    dq 1            ; LBA di partenza

times 510-($-$$) db 0
dw 0xAA55
