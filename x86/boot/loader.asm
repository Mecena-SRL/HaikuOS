; Loader Multiboot2 per la ISO: GRUB carica questo ELF e il kernel come modulo.
; Copia il kernel a 0x100000, scrive le info grafiche a 0x600 (formato di fb.rs),
; attiva paging identità (1 GB + 3..4 GB per il framebuffer) e long mode, salta al kernel.
bits 32

section .text
align 8
mb2_start:
    dd 0xE85250D6                                  ; magic Multiboot2
    dd 0                                           ; architettura i386
    dd mb2_end - mb2_start
    dd -(0xE85250D6 + 0 + (mb2_end - mb2_start))
    align 8
    dw 0, 0                                        ; fine tag
    dd 8
mb2_end:

global _start
_start:
    cli
    mov esp, 0x90000
    mov esi, ebx                                   ; ebx = puntatore alle info Multiboot
    add esi, 8                                     ; salta total_size e reserved
    mov edi, [ebx]
    add edi, ebx                                   ; fine delle info
    xor ebp, ebp                                   ; ebp = 1 se trovato il modulo
    mov dword [0x600], 0                           ; niente grafica finché non trovata
.tag:
    cmp esi, edi
    jae .tags_done
    mov eax, [esi]
    cmp eax, 0
    je .tags_done
    cmp eax, 6                                     ; mappa memoria: copiata a 0x0A00 in formato E820 (UEFI)
    je .mmap
    cmp eax, 3                                     ; modulo: start @8, end @12
    jne .not_mod
    mov eax, [esi + 8]
    mov [mod_start], eax
    mov eax, [esi + 12]
    mov [mod_end], eax
    mov ebp, 1
    jmp .next
.not_mod:
    cmp eax, 8                                     ; framebuffer: addr @8, pitch @16, w @20, h @24, bpp @28
    jne .next
    mov eax, [esi + 8]                             ; layout di fb.rs: base @8, w @12, h @16, pitch @20, bpp @24
    mov [0x608], eax
    mov eax, [esi + 20]
    mov [0x60C], eax
    mov eax, [esi + 24]
    mov [0x610], eax
    mov eax, [esi + 16]
    mov [0x614], eax
    movzx eax, byte [esi + 28]
    mov [0x618], eax
    mov dword [0x600], 0x56424531                  ; magic VBE (riconosciuto da fb.rs)
    jmp .next
; tag 6: entry_size @8 (24), voci da @16 (base u64, len u64, tipo u32, riservato u32)
.mmap:
    push edi
    push esi
    cmp dword [esi + 8], 24
    jne .mmap_end
    mov eax, [esi + 4]
    sub eax, 16
    xor edx, edx
    mov ecx, 24
    div ecx                                        ; eax = numero di voci
    cmp eax, 16
    jbe .mmap_n
    mov eax, 16
.mmap_n:
    mov [0x0A00], eax                              ; COUNT
    mov ecx, eax
    imul ecx, ecx, 24
    lea esi, [esi + 16]
    mov edi, 0x0A10
    rep movsb
.mmap_end:
    pop esi
    pop edi
.next:
    mov eax, [esi + 4]
    add eax, 7
    and eax, 0xFFFFFFF8
    add esi, eax
    jmp .tag
.tags_done:
    cmp ebp, 1
    jne halt

    ; copia del kernel: da mod_start a 0x100000 (all'indietro se le aree si sovrappongono)
    mov esi, [mod_start]
    mov edi, 0x100000
    mov ecx, [mod_end]
    sub ecx, esi                                   ; ecx = dimensione
    cmp edi, esi
    jbe .fwd
    lea eax, [esi + ecx]
    cmp edi, eax
    jae .fwd
    lea esi, [esi + ecx - 1]
    lea edi, [edi + ecx - 1]
    std
    rep movsb
    cld
    jmp .copied
.fwd:
    cld
    rep movsb
.copied:

    ; tabelle di pagina: PML4 0x1000, PDPT 0x2000, PD 0x3000..0x6FFF.
    ; Identità su 0..4 GB con pagine da 2 MB (copre il framebuffer, es. QEMU/OVMF a 0x80000000).
    mov edi, 0x1000
    mov ecx, 6144                  ; 6 pagine da 4 KB, in dword
    xor eax, eax
    rep stosd
    mov dword [0x1000], 0x2003
    mov dword [0x2000], 0x3003
    mov dword [0x2008], 0x4003
    mov dword [0x2010], 0x5003
    mov dword [0x2018], 0x6003
    mov edi, 0x3000
    mov eax, 0x83                  ; presente, scrivibile, pagina da 2 MB
    mov ecx, 2048
.pd:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .pd

    lgdt [gdt_desc]
    jmp 0x08:reload
reload:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov eax, 0x1000
    mov cr3, eax
    mov eax, cr4
    or eax, 0x20                                   ; PAE
    mov cr4, eax
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100                                  ; LME
    wrmsr
    mov eax, cr0
    or eax, 0x80000000                             ; PG
    mov cr0, eax
    jmp 0x18:lm64

halt:
    hlt
    jmp halt

bits 64
lm64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, 0x90000
    mov rax, 0x100000
    jmp rax

section .data
align 8
gdt:
    dq 0
    dq 0x00CF9A000000FFFF                          ; 0x08: codice 32 bit
    dq 0x00CF92000000FFFF                          ; 0x10: dati piatti
    dq 0x00AF9A000000FFFF                          ; 0x18: codice 64 bit
gdt_end:
gdt_desc:
    dw gdt_end - gdt - 1
    dd gdt

section .bss
align 4
mod_start: resd 1
mod_end:   resd 1
