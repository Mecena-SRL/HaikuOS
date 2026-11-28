# Studio dei kernel esistenti per HaikuOS

> **Sintesi in italiano.** Abbiamo esaminato come FreeBSD, OpenBSD, NetBSD, illumos e Linux risolvono schermo, input, memoria, scheduling, interrupt, storage e accelerazione grafica. Per ogni idea utile: cosa fa, perché ci serve, come rifarla da zero nel nostro design. Nessun codice copiato. Le idee più utili per noi sono: schermo come superficie semplice con buffer condivisi e flip sincronizzato al vsync, eventi di input in coda a dimensione fissa, allocatore a pagine con pool a dimensione fissa, scheduling a quote (coerente con il monolite che approva), e la possibilità di eseguire i driver anche fuori dal kernel per testarli sull'host (idea NetBSD "anykernel"). Piano in quattro fasi, a partire dalla Game Boy Advance.

> **Stato delle fonti.** Questo documento riassume la conoscenza di progetto che abbiamo, non una rilettura dei sorgenti in questa passata. Ogni nome di file, driver o funzione citato è **da verificare** prima di qualsiasi implementazione.

## 1. Metodo e vincoli

- **Solo idee.** FreeBSD e OpenBSD/NetBSD sono BSD; Linux è GPLv2; illumos è CDDL. Leggiamo per capire il problema, non per copiare righe o strutture.
- **Clean room.** Ogni idea viene riscritta a partire da specifiche pubbliche (datasheet, specifiche UEFI, VBE, USB HID, virtio, ACPI, manuali CPU) e dal comportamento osservato, con nomi e tipi nostri.
- **Vincoli HaikuOS.** Rust `no_std`, un microkernel per risorsa (vedi [resource-microkernels](resource-microkernels.md)), monolite che approva le quote entro limiti fissi, nessuna allocazione dinamica nel percorso caldo, primo target GBA (~288 KB di RAM).

## 2. Tabella delle idee

Priorità: **P1** serve alla prima versione; **P2** serve dopo; **P3** serve per hardware reale; **P4** rinviabile.

| # | Area | Idea | Da dove (studio) | Cosa fa | Perché per noi | Rifacimento nativo | Priorità |
|---|------|------|------------------|---------|----------------|--------------------|----------|
| 1 | Schermo | Framebuffer da firmware (GOP/VBE) | Linux `efifb`/`simpledrm`, BSD `wsdisplay` | Ottiene modo, pitch, formato dal firmware | Base su x86-64/ARM64 senza driver grafico | Driver `display` con tre campi: modo, pitch, formato | P1 |
| 2 | Schermo | Modello a tre oggetti (plane, output, pipe) | Linux DRM/KMS | Separa cosa si disegna, dove va, come si collega | Ci basta un solo piano e un solo output, ma il modello resta estendibile | Un solo piano per ora; interfaccia pronta per più piani | P2 |
| 3 | Schermo | Oggetti buffer con handle e conteggio riferimenti | Linux GEM | Buffer condiviso tra processi senza copia | Il compositore riceve la finestra senza copiarla | Handle opachi, conteggio nel monolite, nessun puntatore fra processi | P1 |
| 4 | Schermo | Flip di pagina sincronizzato al vsync | Linux DRM vblank, BSD `vt` | Cambia buffer solo nel ritorno verticale | Niente tearing; il frame è pronto entro il tempo fisso | Coda di flip con un solo slot in attesa | P1 |
| 5 | Grafica | Blit 2D di rettangoli e riempimento | Linux `drm` damage clips, BSD `vt` | Copia o riempie un rettangolo con eventuale alpha | Cuore del compositore e dei programmi GBA | Funzioni pure su slice; su GBA via DMA3 | P1 |
| 6 | Grafica | Aggiornamento per regioni sporche | Linux damage clips | Ridisegna solo il rettangolo cambiato | Su GBA la CPU è lenta: meno pixel = più frame | Lista fissa di rettangoli sporchi per frame | P1 |
| 7 | Grafica | Rasterizzazione vettoriale a copertura | stb_truetype, font-rs, rasterizzatori Skia | Calcola l'area coperta di ogni pixel del bordo | Font e icone scalabili senza GPU | Scrittura nostra da formule matematiche; vedi [vector-render-api](vector-render-api.md) | P2 |
| 8 | Input | Coda di eventi con timestamp a dimensione fissa | Linux `evdev`, OpenBSD `wscons` | Trasporta tasti e puntatore con ordine e tempo | Stesso formato per tastiera GBA, PS/2, USB | Anello di N eventi, nessuna allocazione | P1 |
| 9 | Input | Tastiera e mouse PS/2 (i8042) | Linux `i8042`, OpenBSD/NetBSD `pckbc` | Macchina a stati sui byte di scansione | Primo input su x86 senza USB | Driver piccolo, testato sull'host | P2 |
| 10 | Input | Parser di report descriptor HID | Linux `hid`, FreeBSD/OpenBSD `uhid` | Legge la mappa dei dati di un dispositivo USB | Un solo parser per tutte le periferiche USB | Parser puro, nessun I/O, testabile | P3 |
| 11 | USB | Controller xHCI con anelli di comando e eventi | Linux `xhci`, FreeBSD `xhci` | Gestisce trasferimenti e interrupt | Quasi tutti i PC moderni | Solo dopo HID; aiuta la stima dei tempi | P3 |
| 12 | Memoria | Allocatore a pagine (buddy) | Linux, FreeBSD | Assegna blocchi di pagine potenza di due | Base per la memoria fisica | Buddy su bitmap fissa | P1 |
| 13 | Memoria | Pool a dimensione fissa per oggetti kernel | Linux slab, FreeBSD UMA | Cache di oggetti della stessa dimensione | Niente frammentazione e niente heap nel kernel | Pool con dimensione decisa a compilazione | P1 |
| 14 | Memoria | Pagine con W^X e ASLR del kernel | OpenBSD (W^X, KARL) | Una pagina non è mai eseguibile e scrivibile | Sicurezza in priorità alta; vedi [memory-security](memory-security.md) | Regola in fase di mappatura, verifica nei test | P1 |
| 15 | Sicurezza | Promessa e limitazione per processo (pledge/unveil) | OpenBSD | Il programma dichiara cosa può fare e il kernel lo fa rispettare | Coerente con il microkernel per programma | Capacità nel contratto di avvio del programma | P2 |
| 16 | Scheduling | Quote e code per CPU | FreeBSD ULE, illumos fair share | Assegna tempo in base a priorità e quota | Il monolite approva le quote entro limiti fissi | Quote fisse, nessun calcolo in virgola mobile | P2 |
| 17 | Scheduling | Tempo virtuale (EEVDF/CFS) | Linux EEVDF | Dà il tempo al processo più in ritardo | Utile solo con molti processi interattivi | Rinviato: complica il monolite | P4 |
| 18 | Interrupt | Gestore breve più lavoro differito | Linux softirq/tasklet, BSD `taskqueue` | Il gestore copre solo il minimo | Interrupt brevi, nessuna attesa nel gestore | Coda di lavoro fissa dopo il gestore | P2 |
| 19 | Test | Driver eseguibili fuori dal kernel | NetBSD rump/anykernel (concetto) | Stesso codice driver in kernel o in programma di test | Testiamo i driver sull'host, come già fa `sim/` | Driver come libreria con due adattatori | P2 |
| 20 | Storage | Pila a strati (GEOM) | FreeBSD GEOM | Ogni strato trasforma le richieste di blocco | Cifratura, cache, RAID con la stessa interfaccia | Strati come microkernel per risorsa (storage) | P3 |
| 21 | Storage | Controller AHCI e virtio-blk | Linux, FreeBSD | Accesso a disco reale o virtuale | Su QEMU parte prima virtio | virtio-blk prima, AHCI dopo | P3 |
| 22 | GPU | Anelli di comandi con campanello | Linux amdgpu/i915, virtio-gpu | Il programma scrive comandi, la GPU li legge | Base dell'accelerazione reale | Anello fisso per dispositivo; rinviato | P3 |
| 23 | GPU | Fence per attendere il completamento | Linux dma-fence | Segnala quando un comando è finito | Sincronizza compositore e GPU | Contatore monotono per dispositivo | P3 |
| 24 | Discovery | ACPI e device tree | Linux, FreeBSD, NetBSD | Scopre cosa c'è sulla scheda | x86-64 usa ACPI, ARM64 usa device tree | Parser minimo, solo per i dispositivi usati | P3 |
| 25 | Osservabilità | Tracce a buffer circolare | illumos DTrace (concetto), Linux ftrace | Registra eventi a costo basso | Debug senza strumenti pesanti | Buffer fisso con etichette numeriche | P4 |

## 2b. Stato driver e input

- **Riga 8 (coda eventi):** già presente in `sim/src/input.rs` (coda fissa, conta gli scarti). Manca ancora il timestamp.
- **Riga 9 (PS/2):** decodifica spostata in `ps2/` (crate `no_std`, senza I/O), usata dal kernel x86 in `x86/kernel/src/input.rs`. Test host in `ps2/src/lib.rs`.
- **Correzione:** il pacchetto mouse sottraeva 256 ai valori negativi (dx=-2 diventava -258). Ora il byte è già con segno.
- **Riga 8 (coda eventi):** crate `evq/` (`haiku-evq`), coda generica a N slot con timestamp, scarti contati. Test host.
- **Riga 10 (HID):** crate `hid/` (`haiku-hid`), parser puro del report descriptor in array fisso. Test con la tastiera boot.
- **Riga 11 (xHCI):** crate `xhci/` (`haiku-xhci`), TRB a 16 byte, anello comandi con Link e bit di ciclo, lettura eventi. Nessun MMIO ancora.
- Riga 9: già coperta da #287 (`ps2/`), nessuna modifica.
- Righe 18, 20, 21 (P2/P3): non toccate.

## 3. Cosa scartiamo

- **Compatibilità con driver altrui (LinuxKPI, drm-kmod).** Porta molto codice Linux dentro il kernel. Contrario alla leggerezza e alla licenza.
- **Heap generico nel kernel.** Causa frammentazione e tempi imprevedibili. Usiamo pool fissi (riga 13).
- **Scheduler a tempo virtuale completo, in fase iniziale.** Troppa complessità per il primo target (riga 17).
- **Trasferimenti di memoria tra processi con copia.** Usiamo handle (riga 3).

## 4. Ordine di priorità

1. **P1, primo valore (GBA e schermo firmware):** righe 1, 3, 4, 5, 6, 8, 12, 13, 14.
2. **P2, sistema usabile su x86-64 e ARM64 in QEMU:** righe 2, 7, 9, 15, 16, 18, 19.
3. **P3, hardware reale:** righe 10, 11, 20, 21, 22, 23, 24.
4. **P4, rinviabile:** righe 17, 25.

## 5. Piano a fasi

**Fase A — GBA (primo target).**
- Modo framebuffer, blit con DMA3 (riga 5), dirty rect (riga 6), tastiera come sorgente di eventi (riga 8), timer per il tempo di frame.
- Criterio di uscita: un frame aggiornato solo nelle zone sporche, misurato a 60 Hz.

**Fase B — x86-64 e ARM64 in QEMU.**
- Framebuffer da firmware (riga 1), flip sincronizzato (riga 4), oggetti buffer (riga 3), PS/2 (riga 9), schermo 2D virtio senza anelli di comandi (riga 22 rinviata), allocatori (righe 12, 13), W^X (riga 14).
- Criterio di uscita: desktop a 1920x1080 in QEMU con input.

**Fase C — hardware reale e scheduling a quote.**
- Parser HID (riga 10), xHCI (riga 11), virtio-blk e AHCI (righe 21, 20), quote (riga 16), gestore breve più lavoro differito (riga 18), test con driver fuori dal kernel (riga 19).
- Criterio di uscita: tastiera USB e disco funzionanti su una scheda reale.

**Fase D — accelerazione GPU.**
- Anelli di comandi (riga 22), fence (riga 23), memoria video gestita dal monolite. Decisione hardware rinviata: si sceglie quale GPU supportare solo dopo la fase C.

## 6. Da verificare

- Ogni riga marcata "da verificare" nei sorgenti prima di implementare.
- Quale GPU reale supportare per primo (fase D).
- Se il test con driver fuori dal kernel (riga 19) basta con la simulazione `sim/` o serve un runner dedicato.
- Limiti di RAM del GBA per i buffer di blit (riga 5 e 6): misurare su hardware.

## 7. Riferimenti interni

- [resource-microkernels](resource-microkernels.md): modello dei microkernel per risorsa.
- [graphics-acceleration](graphics-acceleration.md): opzioni di accelerazione.
- [vector-render-api](vector-render-api.md): API per il disegno vettoriale.
- [memory-security](memory-security.md): regole di memoria e sicurezza.
- [x86-boot](x86-boot.md): avvio su x86.
- [feasibility-gba](feasibility-gba.md): fattibilità GBA.
