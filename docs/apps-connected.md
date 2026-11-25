# App connesse e utility hardware

> Stato: piano. Nessun codice per hardware reale. Il prototipo è solo host: `sim/src/mail.rs`, `sim/src/contacts.rs`, test in `sim/tests/app_dati.rs`. Ogni ipotesi non verificata è segnata **da verificare**.
> Legato a [resource-microkernels.md](resource-microkernels.md) (rete, periferiche, GPU), [portability.md](portability.md) (GBA / ARM64 / x86-64) e [base-programs.md](base-programs.md) (programmi base).

## 1. Principio

- Un'app non parla mai con l'hardware. Chiede dati o azioni al microkernel della risorsa (rete, GPU, input, storage) tramite il monolite.
- Le app di rete passano sempre dallo stack TCP/IP. Nessuna app accede al link layer.
- Su GBA non c'è rete: ogni app di rete è **assente** o **offline** (usa solo dati locali). Il GBA non ha hardware di rete, quindi non è un problema di software.
- Un'app è un processo con una superficie (vedi base-programs). Se manca una risorsa, l'app mostra lo stato e resta usabile.

## 2. Tabella delle app

Legenda GBA: **no** = non possibile; **offline** = funziona solo con dati locali; **sì** = possibile.
Priorità: **P1** primo ciclo; **P2** sistema utilizzabile; **P3** più avanti.

| # | App | Risorse necessarie | Su GBA | Su ARM64 / x86-64 | Priorità | Note |
|---|---|---|---|---|---|---|
| 1 | Editor di testo | storage | sì | sì | P1 | Già sul GBA (base-programs, voce 3) |
| 2 | Browser | rete, GPU/framebuffer, input | no | sì, per fasi | P2 | Vedi §4. Fase 1 solo testo |
| 3 | Mail | rete (SMTP/IMAP), storage, rubrica | offline (lettura locale) | sì | P2 | Modello dati fatto (§5) |
| 4 | Messaggi | rete, notifiche | no | sì | P3 | Protocollo da scegliere. **da verificare** |
| 5 | App telefono | telefonia (modem/SIM), audio, rubrica | no | solo su hardware con modem | P3 | Dipende dall'hardware del telefono. **da verificare** |
| 6 | Mappe | rete, GPU, GPS | no | sì (senza GPS: solo ricerca) | P3 | Dati di mappa offline sono grandi. **da verificare** |
| 7 | Rubrica | storage | offline | sì | P2 | Modello dati fatto (§5) |
| 8 | Condivisione schermo | rete, GPU (cattura), codec | no | sì, solo con banda e CPU adeguate | P3 | Richiede codifica video. **da verificare** |
| 9 | Gestione stampante | rete o USB, storage | no | sì, con driver stampante | P3 | Protocollo IPP su rete. **da verificare** |
| 10 | Configuratore MIDI | USB (periferica), audio/MIDI | sì, solo se il GBA ha porta link (pochi byte) | sì, con USB | P3 | Su GBA: solo via cavo link, lento. **da verificare** |
| 11 | Colorimetro digitale | periferica USB o sensore | no | sì, solo con sensore collegato | P3 | Driver del sensore specifico. **da verificare** |
| 12 | Foto da fotocamera/video collegata | USB (periferica), storage, GPU | no | sì | P3 | Protocollo PTP/UVC. **da verificare** |
| 13 | Compressione e archivi (tipo WinRAR) | storage, crittografia | sì (LZSS già presente) | sì | P2 | Lettura e cifratura con Ascon/AES. Vedi §6 |
| 14 | Lettore audio/video | audio, storage, GPU | solo audio semplice | sì | P2 | Il video richiede codec. **da verificare** |
| 15 | Scacchi | nessuna | sì | sì | P1 | Solo CPU, ottimo primo test |
| 16 | Promemoria | timer, storage | sì | sì | P1 | Usa l'orologio di sistema |
| 17 | Note | storage | sì | sì | P1 | Riusa l'editor |
| 18 | Calendario | storage, orologio, rete (sincronizzazione opzionale) | offline | sì | P2 | Sincronizzazione: formato da scegliere. **da verificare** |
| 19 | Orologio | timer | sì | sì | P1 | Già in base-programs, voce 6 |
| 20 | Libreria musicale | storage, audio | sì (file piccoli) | sì | P2 | |
| 21 | Libreria fotografica | storage, GPU | sì (poche immagini) | sì | P2 | |
| 22 | Lettore di foto e documenti | storage, GPU | sì (immagini a 15 bit) | sì | P2 | Vedi base-programs, voce 5 |
| 23 | Gestore password | crittografia, storage | sì | sì | P2 | Cifratura con slot, vedi memory-security |
| 24 | Dizionario | storage | sì (dati ridotti) | sì | P3 | Dati offline; lookup online opzionale |
| 25 | Ebook | storage, GPU | sì (testo) | sì | P3 | Formato da scegliere. **da verificare** |
| 26 | Utility di sistema (info, dischi, periferiche) | monolite (stato), storage, input | sì (stato base) | sì | P2 | Legge solo lo stato, non cambia nulla |
| 27 | Terminale | storage, input, GPU | sì (base-programs, voce 7) | sì | P1 | |
| 28 | Editor di script | storage, input | sì | sì | P2 | Linguaggio da scegliere. **da verificare** |
| 29 | Screenshot | GPU (lettura framebuffer) | sì (VRAM) | sì | P2 | |
| 30 | Impostazioni di sistema | monolite, storage | sì | sì | P1 | Include rete e periferiche |

## 3. Raggruppamento per risorsa

- **Solo locali (P1, offline ovunque):** editor, note, scacchi, promemoria, orologio, terminale, impostazioni, compressione, gestore password, libreria musicale e fotografica, lettore foto.
- **Rete (P2–P3):** browser, mail, messaggi, mappe, condivisione schermo, gestione stampante, calendario (sync).
- **Periferiche (P3):** app telefono, configuratore MIDI, colorimetro, foto da camera/video collegata.
- **GPU (P2–P3):** browser, mappe, lettore video, condivisione schermo.

Conseguenza: le app locali si possono fare subito sul GBA e sul simulatore; le app di rete e periferica si fanno sul simulatore con un modello dati e un client fittizio, poi sull'hardware reale.

## 4. Browser: versione per fasi

- **Fase 1, testo:** scarica una pagina (HTTP/1.1, TCP, TLS escluso), estrae il testo, lo impagina. Nessun JavaScript, nessuna immagine. Dimensione pagina con limite fisso.
- **Fase 2, solo testo (scelta di Ivan, via coordinatore):** titoli, paragrafi, liste, link. Nessuna immagine, nessun CSS, nessun form.
- **Fase 3, immagini e moduli:** immagini semplici (GIF o bitmap), form semplici, TLS 1.3 (rischio: RAM e crittografia su macchine piccole). **da verificare** la RAM necessaria.
- **Fuori scope:** JavaScript, CSS completo, video web.

Decisione presa: fase 2 solo testo, come consigliato. Le immagini vengono dopo.

## 5. Prototipo nel simulatore

- `sim/src/mail.rs`: casella a capacità fissa (64 messaggi), controllo indirizzi, corpo massimo 2 KB, segna come letto, ordine dal più recente. Nessun invio reale: l'invio passerà dallo stack di rete.
- `sim/src/contacts.rs`: rubrica a capacità fissa (128 contatti), nome, fino a 4 numeri e 4 indirizzi, ricerca per prefisso, ricerca per numero o mail (per chiamate e mail in arrivo).
- Test: `sim/tests/app_dati.rs` (7 test, passati). Tutti i test del simulatore passano.

## 6. Compressione e cifratura

- Il formato di archivio è un'estensione del modulo LZSS già presente (`sim/src/lzss.rs`), con cifratura Ascon per file e cartelle (`sim/src/crypto.rs`). **da verificare** il formato di contenitore e la compatibilità con ZIP.
- Su macchine potenti: LZ4/Zstd e AES. **da verificare** la velocità.

## 7. Utility hardware

- Stampante, MIDI, colorimetro, camera: ognuna è un driver della risorsa periferica, con capacità dichiarate (manifest). Se il driver manca, l'app mostra "periferica non trovata".
- Nessun driver specifico entra nel codice comune (resource-microkernels §7).

## 8. Da verificare

- Protocollo di messaggi e telefonia su ARM64 e x86-64.
- Formato dei calendari e delle ebook.
- RAM necessaria per TLS e browser fase 3.
- Driver USB per MIDI, colorimetro, camera: quali classi di dispositivo.
- Linguaggio dell'editor di script.
- Formato archivi e compatibilità ZIP.
