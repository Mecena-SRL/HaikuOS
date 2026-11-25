# Fattibilità su Game Boy Advance

> Verifica documentale. Nessun codice per l'hardware, nessun bootloader. Le ipotesi aperte sono in fondo.

## Esito in breve

1. **Limiti di memoria: regge.** Il pool utile è di 288 KB (EWRAM 256 KB + IWRAM 32 KB). Con 4 slot di cui uno sempre libero, i programmi concedibili sono 3. Totale: 64 + 16 + 3 × 32 = **176 KB**, restano **112 KB**. Il valore "208 KB" di `architecture.md` §6 conteggia 4 slot pieni e viola la regola dello slot libero: va corretto.
2. **Rust no_std su ARM7TDMI: fattibile, con rischio toolchain.** Il target è `thumbv4t-none-eabi` (tier 3), con `core` e `alloc` in modalità `no_std`. Serve Rust nightly con `-Z build-std=core`. Dimensioni dei binari: non misurate.
3. **Messaggio da 64 byte: adatto.** Sono 16 parole da 32 bit. Stima non misurata: copia nell'ordine di centinaia di cicli, cioè decine di microsecondi a 16,78 MHz.

## 1. Memoria

| Voce | Fonte |
|---|---|
| CPU ARM7TDMI, 16,78 MHz | [felixjones GBA tutorial](https://felixjones.github.io/gba-tutorial/01_introduction/architecture.html) |
| IWRAM 32 KB, bus 32 bit, dentro il package della CPU | idem |
| EWRAM 256 KB, bus 16 bit, esterna alla CPU | idem |
| VRAM 96 KB, palette 1 KB, OAM 1 KB (fuori dal pool) | idem |
| Bus cartuccia 16 bit, ROM fino a 32 MiB, nessuna cache | idem |

Conseguenze per il modello:

- **Il monolite (64 KB) non entra in IWRAM (32 KB).** Va in EWRAM. In IWRAM restano solo ciò che serve più in fretta: gestori di interrupt e il percorso caldo del dispatch dei messaggi.
- **Il codice conta nel pool.** Kernel, monolite, agente e programmi vengono copiati in RAM dalla ROM, perché l'esecuzione da cartuccia è più lenta (bus a 16 bit, wait state).
- **Slot a 32 KB.** Basta per un programma piccolo. Per giochi più grandi serve verificare caso per caso.

## 2. Rust no_std su ARM7TDMI

Fonti:

- [Rust platform support, armv4t-none-eabi](https://doc.rust-lang.org/rustc/platform-support/armv4t-none-eabi.html): target `thumbv4t-none-eabi`, tier 3, `core` e `alloc` in `no_std`. L'architettura supporta solo Thumb-1, non Thumb-2.
- [lokathor, gba-from-scratch ex1](https://lokathor.github.io/gba-from-scratch/ex1.html): build con nightly, `-Z build-std=core`, `#![no_std]`, `#![no_main]`, panic handler, `_start` in assembly, linker script con `ewram` a `0x2000000` (256 KB), `iwram` a `0x3000000` (32 KB), `rom` a `0x8000000`.

Note:

- L'esempio di lokathor è del periodo in cui uscivano le sue guide. Va rifatto con il toolchain attuale prima di fidarsene.
- Non risulta dalle fonti letture se serva un panic handler custom o `compiler-builtins` extra: da verificare in prototipo.
- Dimensione del binario minimo: non trovata nelle fonti, non misurata.

**Alternativa C.** Toolchain maturo: `arm-none-eabi-gcc` con `-mthumb -march=armv4t`, e librerie GBA esistenti. Non ho misurato se produce binari più piccoli di Rust, quindi non posso dire che sia più leggera. È il piano B già previsto in `architecture.md`.

Giudizio: Rust resta la scelta. Il rischio è il toolchain nightly e la dimensione del codice, entrambi verificabili con un prototipo di build (senza hardware).

## 3. Messaggio da 64 byte

- 64 byte = 16 parole da 32 bit. Una copia con `ldm`/`stm` costa poche decine di istruzioni.
- A 16,78 MHz (un ciclo circa 60 ns) la copia è nell'ordine di decine di microsecondi, stima non misurata.
- Un ring buffer per slot da 16 KB tiene 256 messaggi. Entra nel limite di 32 KB.
- 64 byte bastano per un'intestazione (mittente, destinatario, tipo, lunghezza) e un payload corto. Per dati grandi serve un canale separato, da progettare.

## 4. Ipotesi ancora aperte

1. Build Rust `thumbv4t-none-eabi` con toolchain nightly attuale: non provato.
2. Dimensione reale di kernel, monolite e agente: non misurata.
3. Costo reale del dispatch dei messaggi su ARM7TDMI: stima, non misurata. Serve emulatore (mGBA) o hardware.
4. Esecuzione da ROM contro copia in RAM: tempi non misurati.
5. Il pool di 288 KB non include VRAM, palette e OAM, usati dalla grafica. Vanno considerati nel budget del desktop.

## 5. Note sul repository

- Su `main` esiste solo la cartella `docs/`. `sim/README.md` non c'è, quindi non l'ho letto.
- Non ho modificato `architecture.md`. La correzione del totale (208 → 176 KB) va fatta in una PR separata, se confermata.

## Fonti

- [Rust platform support: armv4t-none-eabi](https://doc.rust-lang.org/rustc/platform-support/armv4t-none-eabi.html)
- [GBA tutorial: architecture (felixjones)](https://felixjones.github.io/gba-tutorial/01_introduction/architecture.html)
- [gba-from-scratch ex1 (lokathor)](https://lokathor.github.io/gba-from-scratch/ex1.html)
