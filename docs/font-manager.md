# Gestore dei font

Stato: proposta. Tutto ciò che non è verificato è segnato **da verificare**.

## 1. Fonti esterne (sintesi)

- **OpenType / TrueType**: un file è una tabella di intestazione (offset table) seguita da una directory di tabelle con tag, checksum, offset e lunghezza. Le tabelle chiave sono `head`, `maxp`, `cmap`, `glyf`/`CFF`. Fonte: [Microsoft, *The OpenType Font File*](https://learn.microsoft.com/en-us/typography/opentype/spec/otff).
- **TrueType Reference Manual (Apple)**: definisce le stesse tabelle e il loro significato, utile come riferimento storico. Fonte: [Apple, TrueType Reference Manual, cap. 6](https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6.html).
- **BDF (Bitmap Distribution Format)**: formato testuale di font bitmap a dimensione fissa, con righe `STARTCHAR`, `BITMAP`, `ENDCHAR`. Fonte: [X.Org, BDF spec](https://www.x.org/docs/BDF/bdf.pdf).
- **PCF (Portable Compiled Format)**: versione binaria e compilata del BDF, con tabelle per metriche e bitmap. Fonte: [FontForge, PCF format](https://fontforge.org/archive/pcf-format.html).
- **macOS, gestione dei font**: Core Text separa gli ambiti di attivazione (`process`, `session`, `user`, `persistent`). Fonte: [Apple, CTFontManagerScope](https://developer.apple.com/documentation/coretext/ctfontmanagerscope). Font Book permette di installare, attivare e disattivare font, raggrupparli in collezioni e vederne l'anteprima con testo di esempio. Fonte: [Guida di Font Book](https://support.apple.com/guide/font-book/toc).

Conseguenza per il progetto: il formato vettoriale è OpenType/TrueType (solo macchine potenti). Il formato GBA è un formato interno a bitmap, più semplice di BDF/PCF. Non usiamo BDF/PCF come formato di archiviazione.

## 2. Formato interno

### 2.1 Bitmap (GBA e tutto ciò che è a dimensione fissa)

- Glifo 8×8 in 1 bit per pixel: 8 byte a glifo.
- Set ASCII stampabile (95 glifi): 760 byte. **Da verificare** con l'intervallo di caratteri reale.
- Intestazione del file: magic, versione, altezza glifo, numero di glifi, nome (max 32 byte), poi i glifi in ordine di codice.
- Nessuna allocazione dinamica: il file è una tabella statica in ROM o SRAM.

### 2.2 Vettoriale (macchina potente)

- File `.ttf` / `.otf` letti in sola lettura. Il parser non è scritto da noi nella prima versione (**da verificare**: crate `no_std` esistente o parser minimo).
- Il gestore conserva solo metadati: nome di famiglia (`name`), stile, numero di glifi (`maxp`).

### 2.3 Metadati comuni (entrambi i formati)

- `id` (u8), `nome`, `formato` (bitmap o vettoriale), `attivo` (bool).
- Il font di sistema è un solo `id` valido.

## 3. Operazioni

1. **Installa**: copia il file nell'archivio dei font, verifica l'intestazione, rifiuta un nome già presente. Il font è installato ma non attivo.
2. **Anteprima**: disegna un testo di esempio con quel font. Non cambia lo stato.
3. **Attiva / disattiva**: un font attivo può essere usato dal sistema. Limite di font attivi: **da verificare** su GBA (RAM e SRAM).
4. **Collezioni**: gruppi con nome di font (un font può stare in più collezioni). Limite di collezioni e di membri: da verificare.
5. **Font di sistema**: impostato su un font attivo. Il font di sistema non si può disattivare; impostarlo attiva il font.

Ordine di sicurezza: il font di sistema è sempre attivo, quindi l'interfaccia non resta mai senza font.

## 4. Dove vive il codice

- **Gestore dei font**: programma separato in `apps/fontmgr` (prototipo host: comandi da stdin, anteprima, attivazione, collezioni, font di sistema). Non è parte del monolite. Nessuna persistenza ancora.
- **Logica di registro**: `sim/src/fonts.rs` (test in `sim/tests/fonts.rs`). Il codice è scritto in modo da poter diventare `no_std` + `alloc`.
- **Lettore OpenType** in sola lettura: `sim/src/opentype.rs` (famiglia, metriche, cmap formato 4, hmtx, glifi semplici loca/glyf; controlli sui limiti; test in `sim/tests/opentype.rs`). I glifi composti non sono supportati.
- **Rendering**: `sim/src/raster.rs` riempie i contorni (regola non-zero, curve quadratiche appiattite) in un canvas di bit. Anteprima reale in `apps/fontmgr`: a 16 px/em in terminale con `#`.
- **Font GBA**: `gba/src/font.rs` resta il font di sistema di partenza (3×5); il formato 8×8 si aggiunge dopo.

## 5. Verificabile su host

- Installazione, nome duplicato, registro pieno.
- Attivazione, disattivazione, limite dei font attivi.
- Font di sistema: non disattivabile, attiva il font, deve esistere.
- Collezioni: creazione, aggiunta, limite dei membri, id sconosciuti.
- Anteprima solo su font installato.
- Lettura OpenType: intestazioni, magic, tabelle mancanti, file troncati, nome Windows e nome Mac, cmap, glifo mancante.
- Rendering: quadrato sintetico a pixel noti, testo con avanzo, curve quadratiche (area), anteprima via comando.

**Non verificabile su host**: rendering sul GBA, lettura reale di TTF/OTF, interfaccia grafica, comportamento della SRAM.

## 6. Da verificare

- Limiti numerici (font installati, attivi, collezioni, membri). I valori del prototipo sono scelte iniziali.
- Dimensione e posizione dell'archivio dei font su GBA (ROM, SRAM o flash).
- Parser OpenType: legge head, maxp, name, cmap (formato 4), hhea, hmtx, loca, glyf semplici. Niente glifi composti, GPOS/GSUB, hinting, cmap formato 12.
- Estetica dell'anteprima e dell'elenco (decisione di Ivan).
