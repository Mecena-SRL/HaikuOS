; Stage 2: carica il kernel (LBA 33 = intestazione con dimensione, LBA 34.. = kernel)
; direttamente a 0x10000, lo copia a 0x100000 in modo protetto, attiva long mode
; (paging identità sul primo GB) e salta al kernel.
[bits 16]
[org 0x7E00]

%define LOAD  0x1000      ; segmento reale: il kernel viene letto a 0x10000
%define KDEST 0x100000    ; indirizzo fisico finale del kernel

stage2:
    mov [drive], dl
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00

    in al, 0x92           ; A20 attivo
    or al, 2
    and al, 0xFE
    out 0x92, al

    ; intestazione kernel (LBA 33): prima dword = dimensione in byte
    mov eax, 33
    mov cx, 1
    mov dx, 0x2000
    call read_disk
    mov ax, 0x2000
    mov ds, ax
    mov eax, [0]
    xor bx, bx
    mov ds, bx
    mov [ksize], eax

    ; kernel: settori = (ksize + 511) / 512, letti a blocchi da 64 (32 KB) a 0x10000
    mov eax, [ksize]
    add eax, 511
    shr eax, 9
    mov [krem], eax
    mov dword [klba], 34
    mov word [kseg], LOAD
.kload:
    mov ecx, [krem]
    test ecx, ecx
    jz .kdone
    cmp ecx, 64
    jbe .kc
    mov ecx, 64
.kc:
    mov [kchk], ecx
    mov eax, [klba]
    mov dx, [kseg]
    mov cx, [kchk]
    call read_disk
    mov eax, [kchk]
    add [klba], eax
    shl eax, 5                  ; 32 paragrafi (512 byte) per settore
    add [kseg], ax
    mov ecx, [kchk]
    sub [krem], ecx
    jmp .kload
.kdone:

    ; ---- VBE: framebuffer lineare 640x480. Prima 32 bit (0x142), poi 24 bit (0x112).
    ; esito a 0x0600: magic(4) base(4) w(4) h(4) pitch(4) bpp(4); magic 0 = niente grafica
    mov dword [0x0600], 0
    call dispi_try              ; 1280x800x32 (VGA std QEMU/Bochs), se presente
    cmp dword [0x0600], 0x56424531
    je .vbe_done
    mov cx, 0x0142
    call vbe_try
    cmp dword [0x0600], 0x56424531
    je .vbe_done
    mov cx, 0x0112
    call vbe_try
.vbe_done:
    ; ---- mappa memoria BIOS (E820): conteggio a 0x0A00, voci da 0x0A10 (24 byte), max 16
    xor ax, ax
    mov es, ax
    mov dword [0x0A00], 0
    xor ebx, ebx
    mov di, 0x0A10
.e820:
    mov eax, 0xE820
    mov ecx, 24
    mov edx, 0x534D4150
    int 0x15
    jc .e820_done
    cmp eax, 0x534D4150
    jne .e820_done
    inc dword [0x0A00]
    add di, 24
    test ebx, ebx
    jz .e820_done
    cmp dword [0x0A00], 16
    jb .e820
.e820_done:
    xor ax, ax
    mov es, ax

    cli
    lgdt [gdt_desc]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp dword 0x08:pm32


; ---- dispi_try: VGA Bochs/QEMU (porte 0x1CE/0x1CF), 1280x800x32, LFB da PCI 00:02.0 BAR0.
dispi_try:
    mov eax, 0x80001010
    mov dx, 0x0CF8
    out dx, eax
    mov dx, 0x0CFC
    in eax, dx
    and eax, 0xFFFFFFF0
    mov [0x0608], eax
    mov dx, 0x01CE
    xor ax, ax
    out dx, ax                  ; indice 0: ID
    mov dx, 0x01CF
    in ax, dx
    and ax, 0xFFF8
    cmp ax, 0xB0C0
    jne .fail
    mov dx, 0x01CE
    mov ax, 4                   ; ENABLE = 0 prima di cambiare modo
    out dx, ax
    mov dx, 0x01CF
    xor ax, ax
    out dx, ax
    mov dx, 0x01CE
    mov ax, 1
    out dx, ax
    mov dx, 0x01CF
    mov ax, 1280
    out dx, ax
    mov dx, 0x01CE
    mov ax, 2
    out dx, ax
    mov dx, 0x01CF
    mov ax, 800
    out dx, ax
    mov dx, 0x01CE
    mov ax, 3
    out dx, ax
    mov dx, 0x01CF
    mov ax, 32
    out dx, ax
    mov dx, 0x01CE
    mov ax, 4
    out dx, ax
    mov dx, 0x01CF
    mov ax, 0x41                ; ENABLE | LFB
    out dx, ax
    mov dword [0x0600], 0x56424531
    mov dword [0x060C], 1280
    mov dword [0x0610], 800
    mov dword [0x0614], 5120
    mov dword [0x0618], 32
    mov byte [0x061C], 16       ; canali BGRX a 32 bit: rpos, rsize, gpos, gsize, bpos, bsize
    mov byte [0x061D], 8
    mov byte [0x061E], 8
    mov byte [0x061F], 8
    mov byte [0x0620], 0
    mov byte [0x0621], 8
    mov byte [0x0622], 1        ; tipo RGB
.fail:
    ret

; ---- vbe_try: cx = modo VBE (senza bit LFB). Imposta il modo e scrive l'esito a 0x0600.
vbe_try:
    mov [0x0630], cx
    xor bx, bx
    mov es, bx
    mov ax, 0x4F01
    mov di, 0x0700
    int 0x10
    cmp ax, 0x004F
    jne .fail
    cmp byte [es:0x0719], 24
    jb .fail
    mov ax, 0x4F02
    mov bx, [0x0630]
    or bx, 0x4000
    int 0x10
    cmp ax, 0x004F
    jne .fail
    mov eax, [es:0x0728]
    mov [0x0608], eax
    movzx eax, word [es:0x0712]
    mov [0x060C], eax
    movzx eax, word [es:0x0714]
    mov [0x0610], eax
    movzx eax, word [es:0x0710]
    mov [0x0614], eax
    movzx eax, byte [es:0x0719]
    mov [0x0618], eax
    ; canali dal blocco VBE (0x1F..0x24): rpos, rsize, gpos, gsize, bpos, bsize
    mov al, [es:0x0720]
    mov [0x061C], al
    mov al, [es:0x071F]
    mov [0x061D], al
    mov al, [es:0x0722]
    mov [0x061E], al
    mov al, [es:0x0721]
    mov [0x061F], al
    mov al, [es:0x0724]
    mov [0x0620], al
    mov al, [es:0x0723]
    mov [0x0621], al
    mov byte [0x0622], 1
    mov dword [0x0600], 0x56424531
.fail:
    ret

; ---- lettura BIOS: eax = LBA, cx = settori, dx = segmento di destinazione (offset 0) ----
read_disk:
    mov [rd_lba], eax
    mov dword [rd_lba + 4], 0
    mov [rd_cnt], cx
    mov [rd_seg], dx
    mov si, rd_dap
    mov dl, [drive]
    mov ah, 0x42
    int 0x13
    jc disk_fail
    ret

disk_fail:
    mov si, msg_disk
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

; ---- modo protetto 32 bit: copia, paging, long mode ----
[bits 32]
pm32:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov esp, 0x90000

    ; copia del kernel da 0x10000 a 0x100000
    mov esi, 0x10000
    mov edi, KDEST
    mov ecx, [ksize]
    rep movsb

    ; tabelle di pagina a 0x1000 (PML4), 0x2000 (PDPT), 0x3000 (PD)
    mov edi, 0x1000
    mov ecx, 4096          ; 0x1000..0x5000: PML4, PDPT, PD 0..1 GB, PD 3..4 GB
    xor eax, eax
    rep stosd
    mov dword [0x1000], 0x2003
    mov dword [0x2000], 0x3003
    mov dword [0x2018], 0x4003   ; 3..4 GB: framebuffer VBE (QEMU ~0xFD000000)
    mov edi, 0x3000
    mov eax, 0x83            ; presente, scrivibile, pagina da 2 MB
    mov ecx, 512
.pd:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .pd
    mov edi, 0x4000
    mov eax, 0xC0000083
    mov ecx, 512
.pd2:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .pd2

    mov eax, 0x1000
    mov cr3, eax
    mov eax, cr4
    or eax, 0x20             ; PAE
    mov cr4, eax
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100            ; LME
    wrmsr
    mov eax, cr0
    or eax, 0x80000000       ; PG
    mov cr0, eax
    jmp 0x18:lm64

[bits 64]
lm64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, 0x90000
    mov rax, KDEST
    jmp rax

; ---- dati ----
[bits 16]
drive:    db 0
ksize:    dd 0
krem:     dd 0
klba:     dd 0
kseg:     dw 0
kchk:     dd 0
msg_disk: db "Errore disco (stage2)", 0

rd_dap:
    db 0x10, 0
rd_cnt:   dw 0
    dw 0                     ; offset
rd_seg:   dw 0
rd_lba:   dq 0

align 8
gdt:
    dq 0
    dq 0x00CF9A000000FFFF    ; 0x08: codice 32 bit
    dq 0x00CF92000000FFFF    ; 0x10: dati piatti 4 GB
    dq 0x00AF9A000000FFFF    ; 0x18: codice 64 bit
gdt_end:
gdt_desc:
    dw gdt_end - gdt - 1
    dd gdt

times 16384-($-$$) db 0      ; esattamente 32 settori
