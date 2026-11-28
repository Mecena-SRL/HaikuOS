# Stato del progetto

> Fotografia di `main` (PR #262). Tutto ciò che non è stato provato su hardware è **da verificare**. Dettagli nei documenti linkati: questa pagina non li ripete.

## Funziona (verificato in simulatore o test host)

- **Avvio x86-64 in QEMU:** controllo CPU, schermo a 800×600×24 e 1280×800×32 (vedi [architecture.md](architecture.md) §12).
- **ARM64 in QEMU virt:** avvio UEFI, GOP, desktop con 9 icone, launcher, app CALC, file system FAT, rete minima con HTTP GET (vedi [architecture.md](architecture.md) §12).
- **App di sistema, logica:** registri, connessioni, dischi, monitor, meteo, audio, video, timer, scacchi, distruggi file. Test sull'host (vedi [apps-system.md](apps-system.md), [apps-connected.md](apps-connected.md), [apps-media.md](apps-media.md)).
- **Gestore password, core cifrato:** archivio Ascon con sblocco, modifica e salvataggio. Test in `gba/host/test_vault.rs` (vedi [memory-security.md](memory-security.md)).
- **Icone e sfondi:** 60 icone SVG (`icons/`), 5 sfondi HD (`design/wallpapers/`), mockup HD (`design/hd/`).
- **API di rendering:** `rect_rounded`, `line`, `icon_vec`, `menu` nel trait `Canvas` con test (vedi [vector-render-api.md](vector-render-api.md)).
- **Giochi, solo logica:** citybuilder (core e vista grafica), pinball (fisica e punteggio), platformer (test in `games/platformer/tests/logic.rs`).

## Solo mockup o piano

- Mockup HD di desktop, app e icone: non ancora collegati a un'interfaccia reale.
- Compatibilità hardware per classe: piano, senza driver nuovi ([hardware-compatibility.md](hardware-compatibility.md)).
- Desktop options e bibbia grafica: decisioni, non implementazione ([desktop-options.md](desktop-options.md), [graphic-bible.md](graphic-bible.md)).
- Ricompilatore C→Rust: design e prototipo ([recompiler.md](recompiler.md)).
- Office, file system e risorse: piani e prototipi ([office-suite.md](office-suite.md), [filesystem-roadmap.md](filesystem-roadmap.md), [resource-microkernels.md](resource-microkernels.md)).
- Kart: non presente nel repo.

## Serve hardware reale

- **Game Boy Advance:** tutto il codice gira solo in emulatore (mGBA). Nessuna misura su ARM7TDMI reale (vedi [feasibility-gba.md](feasibility-gba.md)).
- **Salvataggio su disco x86:** manca il driver disco. Il ramo AHCI è in corso e va provato su disco SATA reale.
- **Firmware UEFI reale e GPU:** il 16 bit e il GOP reale non sono verificati.
- **Tempi di risposta e memoria reale:** tutte le misure di [architecture.md](architecture.md) §5–6 sono da verificare.

## Prossimi tre passi

1. **x86-64 in modalità lunga:** il kernel è ancora a 32 bit ([x86-boot.md](x86-boot.md)).
2. **Salvataggio su disco:** collegare la configurazione e lo store al file system reale.
3. **Desktop collegato:** portare i mockup HD e le app di sistema nel lanciatore, con i test su framebuffer.

Graphify non è accessibile da questo ambiente: le voci da riportarci sono in [graphify-pending.md](graphify-pending.md).
