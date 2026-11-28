# Portabilità di HaikuOS

> Stato: bozza. Tutto ciò che è segnato **"da verificare"** è un'ipotesi. Test solo sul simulatore host (`sim/`), nessun hardware reale.

Regole di Ivan, da `docs/architecture.md` sezioni 9-11:
1. Ogni microkernel dichiara le architetture supportate.
2. Il livello alto è codice comune; il livello basso è specifico per macchina.
3. Installazione e avvio sono atomici: si carica solo ciò che funziona, o nulla.

## 1. Matrice architetture e componenti

Legenda: **C** = codice comune (Rust `no_std`), **S** = specifico per architettura.

| Componente | Livello | GBA (ARM7TDMI) | ARM64 | x86-64 |
|---|---|---|---|---|
| Monolite, agente, file system, messaggi | alto | C | C | C |
| Filtro messaggi (allowlist) | alto | C | C | C |
| Avvio | basso | S (header ROM, asm) | S (firmware UEFI) | S (firmware UEFI) |
| Timer | basso | S | S (da verificare: timer generico ARM) | S (da verificare: TSC/APIC) |
| Memoria fisica e MMU | basso | S (mappa fissa, nessuna MMU) | S (MMU) | S (MMU) |
| Schermo | basso | S (VRAM) | S (framebuffer UEFI) | S (framebuffer UEFI) |
| Input | basso | S (tastiera GBA) | S (da verificare) | S (da verificare) |
| Cifratura | medio | Ascon-AEAD128 | AES-256-GCM o ChaCha20-Poly1305 | AES-256-GCM o ChaCha20-Poly1305 |

Fonti: `docs/memory-security.md`, `docs/graphics.md`, `docs/process-management.md`.

**Nota sulla compilazione.** Il livello C si compila per una destinazione neutra; il livello S per il target reale (da verificare: target triple `thumbv4t`, `aarch64`, `x86_64`). Il codice C non deve contenere `cfg` per l'architettura: se serve, il dettaglio va nel livello S.

## 2. Regola di rilevamento hardware

All'avvio, il livello basso produce un descrittore `Target { arch, ram, features }`:
- `arch`: fissata dal target di compilazione (GBA) o letta dal firmware (ARM64/x86-64);
- `ram`: letta dal firmware o fissa (GBA: 288 KB);
- `features`: es. AES hardware (da verificare: come interrogarla su ogni architettura).

Ogni componente dichiara:
- `archs`: architetture supportate;
- `required`: se è indispensabile;
- `bytes`: memoria necessaria.

Regole:
- architettura sconosciuta → nulla caricato (fail closed);
- componente `required` incompatibile → installazione bloccata;
- componente opzionale incompatibile → saltato e registrato nel log.

## 3. Modello di installazione atomica

1. **Rilevamento:** costruisce il `Target`. Nessuna modifica.
2. **Verifica:** calcola il piano (componenti compatibili, RAM sufficiente, `required` presenti). Nessuna modifica.
3. **Attivazione:** attiva i componenti del piano in ordine di dipendenza.
4. **Commit:** lo stato attivo cambia in un solo passo. Su hardware reale: un flag di validità scritto per ultimo (da verificare: atomicità della scrittura su flash/SRAM del GBA).
5. **Rollback:** se un'attivazione fallisce, si disattivano quelle già attive in ordine inverso e lo stato resta quello precedente.

Il chiamante aggiorna il proprio stato solo con esito `Ok`. Nel simulatore l'implementazione è `sim/src/install.rs`.

## 4. Test nel simulatore

`sim/tests/portabilita.rs` verifica:
- un componente opzionale incompatibile non viene caricato;
- un componente richiesto incompatibile blocca tutto, senza attivare nulla;
- RAM insufficiente blocca tutto, senza attivare nulla;
- un'attivazione fallita fa rollback in ordine inverso e non lascia stato parziale.

Esito: `cargo test` in `sim/`, 4 test nuovi, tutti passati.

## 5. Da verificare

- Quali componenti sono davvero comuni: il monolite dipende da ordinamento e memoria, che cambiano tra architetture (da verificare).
- Compilazione del livello C per i tre target, con `no_std`: non ancora provata.
- Lettura dell'architettura e delle feature su ogni piattaforma reale.
- Atomicità della commit su flash e su SRAM del GBA.
- Firma o hash dei componenti prima della verifica (da verificare: scelta dell'algoritmo, vedi `docs/memory-security.md`).
- Il simulatore modella solo architettura e RAM, non le feature hardware.
- Tempi di installazione su hardware reale: non misurati.
