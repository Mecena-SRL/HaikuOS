# Suite office di HaikuOS

> Stato: tappa 1 (formato e modulo testo). Ogni ipotesi non misurata è **da verificare**.
> Riferimenti letti: `docs/apps-system.md`, `docs/filesystem-roadmap.md`, `gba/src/fs.rs`, `gba/src/editor.rs`.
> La suite è riscritta da zero. Il repo `artcraft` è letto solo per capire formati e funzioni: nessun codice copiato.

## 1. Obiettivo

Una suite piccola ed efficiente: testo, fogli, presentazioni, PDF in sola lettura.
Prima macchina: GBA (288 KB RAM, file system da 8 KB). Poi ARM64/AMD64 senza cambiare i formati.

## 2. Tappe

1. **Testo** (questa PR): formato, lettura e scrittura di un file di testo, modulo `office/src/text.rs`.
2. Fogli: celle fisse, formule semplici. Formato da decidere dopo la tappa 1.
3. Presentazioni: una diapositiva = una schermata del GBA (240×160).
4. PDF in sola lettura. **Da verificare**: il formato reale va letto dalla specifica ufficiale, non ancora accessibile da questo sandbox.

## 3. Scelta del formato

| Formato | Scelta | Motivo |
|---|---|---|
| `.txt`, byte puri, a capo LF | **Sì, formato nativo v1** | Nessun parser, nessuna dipendenza. Entra in un blocco di file system (`fs::MAX_FILE` = 8 KB). |
| `.docx` (Word) | No, per ora. Solo import in tappa futura | È un archivio ZIP compresso con XML dentro. Serve un inflate e un parser XML: troppo per GBA. **Da verificare** dimensione del codice. |
| `.odt`, `.xlsx`, `.pptx` | No, per ora | Stessa struttura ZIP+XML di `.docx`. |
| Formato proprio binario | No | Non serve: il testo semplice è leggibile anche fuori dall'OS. |

Regole del formato v1:
- Solo byte. Il testo è Latin-1/ASCII. **Da verificare**: caratteri accentati nel font GBA.
- Fine riga = LF. In lettura CRLF e CR diventano LF. In scrittura si scrive sempre LF.
- Nessuna intestazione, nessun metadato. Un file `.txt` si apre con qualsiasi editor.
- Dimensione massima: 8 KB (un file del file system).

## 3b. Compatibilità con le suite esistenti

Obiettivo di Ivan: leggere e scrivere con Microsoft Office, LibreOffice, OpenDocument, Pages e la suite Apple.

| Formato | Dove | Stato |
|---|---|---|
| `.txt` | GBA e macchine potenti | Nativo, tappa 1 |
| `.docx` (Word) | Macchine potenti | Lettura e scrittura, tappa futura. ZIP + XML (ECMA-376). **Da verificare** |
| `.odt` (OpenDocument, LibreOffice) | Macchine potenti | Lettura e scrittura, tappa futura. ZIP + XML (ODF). **Da verificare** |
| `.pdf` | Macchine potenti | Sola lettura, tappa 4 |
| `.pages` (Apple) | Macchine potenti | Solo dopo verifica della specifica, formato poco documentato. **Da verificare** |

Limite: il GBA non ha memoria per ZIP e XML completi. La compatibilità piena parte dalle macchine potenti. Decisione (Ivan, 2026-11-26): compatibilità piena sulle macchine potenti. Il GBA usa solo `.txt`.

## 3c. Stato compatibilità (tappa 2 in corso)

- `office/src/inflate.rs`: DEFLATE (RFC 1951) senza allocazioni. Test su blocchi fissi, dinamici e stored.
- `office/src/zip.rs`: lettura ZIP (stored e DEFLATE). Scrittura stored con CRC-32. Il CRC non si verifica in lettura (**da verificare**).
- `office/src/formats.rs`: lettura del testo di `.docx` e `.odt`; scrittura di un docx e un odt minimi di solo testo.
- Verifica: LibreOffice apre i nostri docx e odt e ne estrae il testo corretto. Il nostro lettore legge docx e odt creati da LibreOffice.
- Da verificare: caratteri accentati (il file di test usa UTF-8 nei byte, non Latin-1; resta da controllare sul font GBA e sulle suite Apple).
- PDF in sola lettura: solo font semplici (WinAnsi). Font incorporati con Identity-H (quasi tutti i PDF di LibreOffice) richiedono il ToUnicode: rinviato.
- `.pages`: non ancora verificato.

## 4. Modulo testo (`office/src/text.rs`)

- `Text<N>`: buffer fisso di `N` byte, nessuna allocazione, solo `core`.
- Operazioni: `load`, `bytes`, `insert`, `backspace`, `delete`, `left`, `right`, `up`, `down`, `line_col`, `lines`, `line`.
- Errori espliciti: `Full` (nessuno spazio, il contenuto non cambia) e `BadChar` (CR in inserimento).
- Il modulo non conosce il file system: legge e scrive solo byte. Chi lo usa passa i byte da `Fs` (`gba/src/fs.rs`).

Test host (8, tutti ok):

```sh
cd office && rustc --edition 2021 --test src/text.rs -o /tmp/office_text_test && /tmp/office_text_test
```

## 5. Differenze rispetto all'editor esistente

`gba/src/editor.rs` gestisce già un testo, ma è legato alla tastiera e alla sua schermata. `office/text.rs` è una base separata e riusabile da più app (editor, note, fogli). Quando la suite sarà pronta, l'editor potrà usare questo modulo. Da decidere con Ivan.

## 6. Da verificare

- Consumo reale di RAM e CPU su ARM7TDMI (costo di `insert` con `copy_within` su 8 KB).
- Caratteri accentati nel font del GBA.
- Specifica di `.docx` e `.pdf` (per le tappe future).
