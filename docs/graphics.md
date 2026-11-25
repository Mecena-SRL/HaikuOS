# Ambiente grafico di HaikuOS

> Documento di ricerca e design. Ogni ipotesi non misurata è segnata **"da verificare"**.
> Legato a [architecture.md](architecture.md): il grafico non deve mai dare al monolite politiche di priorità.

## 1. Obiettivo

Un ambiente grafico che:
- parta dalla Game Boy Advance (240×160, 15 bit a colori, circa 288 KB di RAM) e scali a ARM64 e AMD64;
- isoli il disegno tra programmi (un programma non disegna sopra la finestra di un altro);
- sia piccolo: ogni livello è testabile da solo.

## 2. Stato dell'arte (sintesi)

| Sistema | Idea chiave | Cosa prendiamo | Cosa scartiamo |
|---|---|---|---|
| **Wayland** (protocollo) | Il compositor possiede i buffer, i client disegnano da soli e consegnano buffer | Modello "client disegna, compositor compone" | Stack troppo grande per GBA; dipende da molte librerie |
| **Redox Orbital** | Server di display in user space; le app parlano via schema/IPC | Display server come programma separato, niente driver nel kernel | Protocollo testuale e client pesanti per i nostri limiti |
| **Fuchsia Flatland** (Scenic) | Ogni vista è isolata tramite token; il compositor compone grafi di scena 2D; i buffer passano da sysmem | Isolamento per "token di vista" e scena dichiarativa | Complessità del modello di sicurezza e del GPU stack |
| **Haiku app_server** | Un server separato; le app inviano comandi di disegno via porte | Separazione server/client e comandi di disegno, non pixel | Dipendenza da un'API grande (BView, BWindow) |
| **SerenityOS (WindowServer)** | Server di finestre in user space con protocollo IPC semplice | Forma semplice del protocollo, finestre come proprietà del server | Gran parte del codice è C++ con runtime pesante |

Fonti: [Wayland e Weston](https://chromium.googlesource.com/external/anongit.freedesktop.org/git/wayland/weston/+/3100d3635f03df0d7db7d75d9bb00cb5e9a64791/README.md), [Redox Graphics and Windowing](https://doc.redox-os.org/book/ch04-09-graphics-windowing.html), [Fuchsia Scenic](https://fuchsia.dev/docs/concepts/ui/scenic), [Fuchsia Flatland](https://fuchsia.dev/docs/concepts/ui/scenic/flatland), [RFC-0147 View System](https://fuchsia.dev/docs/contribute/governance/rfcs/0147_view_system).

Ricerca fatta solo con risultati di ricerca generici: la descrizione dei sistemi è da verificare sui documenti ufficiali prima di fissare le API.

## 3. Livelli proposti

```
Interfaccia          (widget, testo, finestre: solo nei programmi)
Compositor           (un processo: compone le superfici dei programmi)
Driver schermo       (framebuffer o registri LCD: scrive i pixel)
Monolite / verticale (concede il buffer, verifica i permessi)
```

1. **Driver schermo.** Su GBA scrive in VRAM (modi 3 e 4 per il bitmap, modi 0–2 per i tile). Su macchine grandi usa un framebuffer lineare o il driver GPU.
2. **Compositor.** Riceve superfici (rettangoli con pixel) dai programmi e le mette in ordine. Non conosce il contenuto delle finestre.
3. **Interfaccia.** Libreria che i programmi usano per disegnare. Niente di essa è privilegiato.

Regola: il compositor non legge la memoria dei programmi. Riceve solo superfici concesse dal monolite con quota fissa.

## 4. Per GBA

- **Modi video.** Modo 3 (bitmap 240×160, 15 bit): un solo buffer da 75 KB, troppo per la RAM disponibile se si fanno più finestre. Modo 4 (8 bit con palette, due pagine): circa 38 KB per pagina, più adatto. Modi 0–2 (tile): sfruttano sprite e tile, più adatti a giochi. **Da verificare** con gbatek.
- **Plan di base.** Un programma = una superficie = un livello di tile o una porzione del modo 4. Il compositor decide l'ordine, non i pixel.
- **Sprite.** 128 sprite hardware, fino a 64×64. Il cursore e le icone vanno qui, non nel buffer.
- **Sincronizzazione.** Disegno solo durante il blank verticale (circa 60 Hz) per evitare tearing. **Da verificare** il tempo utile.
- **Stato.** La ROM attuale usa il modo 3 (vedi sezione 6).
- **Cosa è già possibile.** Mostrare una finestra per programma con quota fissa (32 KB, come da architecture.md), spostare sprite come cursore. Non è possibile un compositor con alpha blending completo nel tempo reale senza misure: **da verificare**.

## 5. Per macchina potente

- Framebuffer lineare dal bootloader (UEFI GOP su AMD64/ARM64), poi driver GPU per verticale.
- Compositor a buffer: ogni programma consegna un buffer con token; il compositor compone con la GPU se disponibile, altrimenti in CPU.
- Protocollo: messaggi a campi fissi (tipo, id, rettangolo, handle buffer). Nessun parser testuale.

## 6. Stato attuale (da `gba/`, main)

- La ROM usa il modo 3 (`DISPCNT = 0x0403`). Le schermate di boot e misure scrivono direttamente in VRAM.
- **Compositore** (`gba/src/compositor.rs`): copia superfici rettangolari nel framebuffer, con ritaglio ai bordi, ordine a strati e trasparenza con chiave di colore.
- **Superfici dei programmi** (`gba/src/surfaces.rs`): ogni programma apre una superficie 64×48 (6 KB) solo con quota concessa dal monolite (`Quota`, implementato da `Monolith`). Un programma scrive solo nel proprio buffer. Alla chiusura la quota torna.
- **Demo in ROM:** dopo la schermata misure (tasto A), tre programmi (0, 2, 3) disegnano superfici verde, blu e arancio, composte in VRAM. Chiusura alla pressione successiva di A.
- **Verifica:**
  - test host dei moduli: 6 per il compositore, 10 per le superfici (`host/test_surfaces.rs`, quota negata, doppia apertura, ordine);
  - harness libmgba (`gba/host/vram_check.c`): confronta tutti i 38.400 pixel della VRAM al frame 400 con la composizione attesa. Esito: 0 differenze.
- Non verificato su hardware reale. Lo screenshot di mGBA sotto Xvfb non funziona in questo ambiente.

## 7. Domande aperte

1. Linguaggio del compositor: Rust no_std come da decisioni (da confermare per il codice grafico).
2. Il primo programma grafico è un desktop generico o un gioco? Cambia la priorità tra modo 4 e tile.
3. Quota video per programma sulla GBA: 32 KB basta per una finestra in modo 4? **Da verificare**.
