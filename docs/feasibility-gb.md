# Fattibilità sul Game Boy originale (DMG)

> Verifica documentale. Nessun codice per l'hardware, nessun bootloader. Ogni ipotesi non misurata è marcata **"da verificare"**.
> Sostituisce lo studio su Game Boy Advance (`feasibility-gba.md`), che usava numeri sbagliati per il DMG (288 KB di RAM non valgono qui).

## Esito in breve

1. **RAM: lo spazio è molto piccolo.** Tutta la RAM di lavoro è **8 KB** (WRAM). Il modello a 64 KB / 16 KB / 32 KB per slot non entra: va ridisegnato in byte (sezione 1).
2. **Il codice non occupa RAM.** Su DMG il programma gira direttamente dalla cartuccia (ROM). Solo i dati consumano la WRAM. Questo cambia il modello rispetto al GBA, dove si copiava il codice in RAM.
3. **Rust ufficiale: bloccato.** Il DMG (CPU SM83) non ha supporto in rustc, nemmeno Tier 3. Con solo Rust ufficiale il DMG non si può costruire. Opzioni realistiche: (a) rimandare il DMG e partire dal GBA (ARM7TDMI, Rust supportato); (b) scrivere un backend LLVM per SM83, progetto grande, durata **da verificare**.
4. **Messaggio da 64 byte: realistico.** Un ring buffer di 16 messaggi occupa 1 KB, cioè l'8% della RAM.
5. **Agente di allocazione: realistico**, con blocchi fissi e bitmap di pochi byte.

## 1. Memoria reale

Fonte: [Pan Docs, Memory Map](https://gbdev.io/pandocs/Memory_Map.html). Valori dalla documentazione tecnica, da riconfermare sulla pagina.

| Zona | Dimensione | Note |
|---|---|---|
| WRAM (C000–DFFF) | 8 KB | Unica RAM di lavoro, condivisa da tutto |
| VRAM (8000–9FFF) | 8 KB | Solo grafica, non usabile come RAM generica |
| OAM (FE00–FE9F) | 160 B | Sprite, usata dalla grafica |
| HRAM (FF80–FFFE) | 127 B | Veloce, adatta a variabili critiche e stack minimo |
| ROM cartuccia | fino a 1 MB con MBC5 usato nei giochi, più in teoria | Bank 0 fisso, il resto a banchi |
| RAM cartuccia | nella cartuccia | Non fa parte del sistema, serve al salvataggio |

Pokémon Gialla è un esempio: ROM da 1 MB con banchi. Il suo salvataggio sta nella RAM della cartuccia, non nei 8 KB.

**Budget WRAM proposto** (da verificare con prototipo):

| Voce | Byte | Note |
|---|---|---|
| Riserva di sistema e stack | 1024 | Stack in WRAM, HRAM per il minimo |
| Shadow OAM | 160 | Copia degli sprite |
| Monolite (dati) | 512 | Solo contatori e tabelle, codice in ROM |
| Agente di allocazione | 256 | Bitmap e stato |
| 4 slot programma | 4 × 1536 = 6144 | Uno sempre libero, quindi 3 attivi |
| Margine | 256 | Da verificare |
| **Totale** | **8192** | |

Regola "slot libero sempre disponibile": con 4 slot, al massimo 3 programmi attivi da 1536 B. Il vecchio calcolo (4 slot pieni, 208 KB) è sbagliato in ogni caso.

Il vincolo di scala si esprime meglio in byte: il monolite conta i byte, non i KB.

## 2. Toolchain

| Opzione | Stato | Pro | Contro |
|---|---|---|---|
| Rust no_std (rust-z80 fork, escluso) | Fork non upstream | Linguaggio scelto in architecture.md | Nessun supporto stabile, binari e manutenzione da verificare. Fonte: [rust-gb README](https://github.com/zlfn/rust-gb) |
| C con SDCC / GBDK-2020 | Maturo, supporta SM83 e ROM a banchi | Toolchain testata, documentazione | Più verboso di Rust, meno garanzie sulla memoria |
| Assembly (RGBDS o simili) | Maturo | Controllo totale, binari minimi | Lento da scrivere, difficile da mantenere |

Fonti: [GBDK-2020](https://github.com/gbdk-2020/gbdk-2020) (SDCC per sm83). Dimensioni dei binari: **da verificare** (non misurate).

**Decisione di Ivan:** solo Rust originale, niente C e niente fork. Con questo vincolo il DMG è bloccato: non si procede con C. Il fork rust-z80 resta descritto ma escluso. Il DMG va rimandato o affrontato con un backend LLVM dedicato.

## 3. Messaggio da 64 byte e allocazione

- **Messaggio:** 64 B sono 0,8% della WRAM. Una copia con loop di `ld a,(hl+)` / `ld (de),a` costa circa 2500 cicli, cioè circa 0,6 ms a 4,19 MHz. **Stima, da verificare.**
- **Coda:** 16 messaggi = 1 KB. Dentro il budget.
- **Agente di allocazione:** blocchi da 16 B su 1536 B di slot = 96 blocchi, bitmap di 12 B. Su 256 B di agente basta. **Realistico.**

## 4. Proposta di modifica a docs/architecture.md (non ancora applicata)

1. §1 Visione: sostituire "Game Boy Advance (288 KB)" con "Game Boy originale (8 KB di WRAM)". Il GBA resta come obiettivo successivo, non il primo.
2. §5 Admission control: il controllo si fa in byte, non in KB.
3. §6 Limiti: sostituire la tabella con il budget in byte della sezione 1, marcato "da verificare".
4. §2.5 e §2.6: precisare che il codice dei programmi gira da ROM e non conta nella WRAM.
5. §8 Domande aperte: aggiungere "DMG: rimandare, oppure scrivere un backend LLVM SM83?" (decisione per Ivan). Il linguaggio è già deciso: solo Rust.
6. Il file `feasibility-gba.md` va marcato come superato o rimosso in una PR separata.

## 5. Ipotesi da verificare

1. Valori Pan Docs della sezione 1 (riletti da memoria, non da pagina fetchata).
2. Dimensione binari SDCC/GBDK per kernel minimo e monolite.
3. Costo reale del dispatch dei messaggi e del loop di copia.
4. Comportamento del fork rust-z80 su una build di prova.
5. `sim/` su main usa ancora i limiti GBA: non l'ho letto e va allineato.

## Fonti

- [Pan Docs, Memory Map](https://gbdev.io/pandocs/Memory_Map.html)
- [rust-gb (README, fork SM83)](https://github.com/zlfn/rust-gb)
- [GBDK-2020 (SDCC per sm83)](https://github.com/gbdk-2020/gbdk-2020)
- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
