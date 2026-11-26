# Formati Office e OpenDocument (lettura)

> Fase 1, solo lettura di testo. Ogni misura non provata è segnata **da verificare**.
> Base: [compatibility.md](compatibility.md) (formati disco) e [app-media.md](app-media.md) (lettore testo).

## Stato

| Formato | Lettura testo | Test | Note |
|---|---|---|---|
| DOCX | sì | `sim/tests/office.rs` | paragrafi, tabulazioni, a capo, entità XML |
| ODT | sì | idem | titoli, spazi `text:s`, tabulazioni |
| ODS | sì | idem | fogli con nome, righe, celle, celle ripetute |
| XLSX | sì, solo primo foglio | idem | stringhe condivise e inline |
| PDF | testo di base | `pdf_testo_base` | flussi FlateDecode o non compressi; font con codifica propria no. **Da verificare** su PDF reali |
| Pages (`.pages`) | no | - | fase 2: contenuto in IWA (Snappy + protobuf). **Da verificare** |
| DOCX, ODT | scrittura minima (un paragrafo per riga) | `scrittura_docx_odt_round_trip` | solo testo; Word e LibreOffice da provare |

## Moduli (sim/src)

- `inflate.rs`: DEFLATE (RFC 1951), no_std, nessuna allocazione. Output in buffer fisso.
- `zip.rs`: lettore ZIP, metodi 0 e 8, CRC-32 verificato. Niente ZIP64, niente cifratura.
- `xml.rs`: tokenizer XML minimo (tag, testo, attributi). Non valida.
- `pdf.rs`: testo da flussi di contenuto PDF (operatori Td, T*, TJ, Tj).
- `officewrite.rs`: DOCX e ODT minimi, con `zip::write_stored` (ZIP stored + CRC-32).
- `office.rs`: estrazione del testo. Nucleo no_std su `Sink` a buffer fisso; `read_text` (feature `std`) rileva il formato.

## Regole

- Il formato si riconosce dal contenuto: `word/document.xml` (DOCX), `xl/workbook.xml` (XLSX), `mimetype` (ODT/ODS).
- Testo in uscita: UTF-8. Paragrafi e righe a capo, celle a tabulazione.
- Ogni voce estratta ha il CRC verificato. Un file corrotto dà errore, non panico (test con file troncati).

## Limiti (da verificare su file reali)

- XLSX: legge solo `sheet1.xml`, non il primo foglio secondo l'ordine del workbook.
- XLSX: ogni cella con stringa condivisa scandisce `sharedStrings.xml`. Lento per fogli grandi.
- ODS: celle ripetute scrivono il testo una volta e le tabulazioni per tutte le ripetizioni (max 256).
- DEFLATE: decodifica bit a bit, circa lenta. Va misurata sul GBA prima di usarla lì.
- Fixture create con `zipfile` di Python, non con Word, LibreOffice o Pages. Compatibilità reale **da verificare**.
- Nessun supporto a macro, stili, immagini, grafici, commenti, note.

## Da fare

1. Pages: lettore IWA (Snappy a blocchi, poi protobuf per il testo). **Da verificare**.
2. Verifica DOCX/ODT scritti con Word e LibreOffice (non disponibili nel sandbox).
3. PDF: font con ToUnicode, object stream.
