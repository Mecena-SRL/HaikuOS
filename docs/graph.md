# Mappa viva del codice (sim)

Aggiornare a ogni PR. Frecce: "usa".

| Modulo | Ruolo | Usa | Stato |
|---|---|---|---|
| `sim/src/inflate.rs` | DEFLATE no_std | - | fatto, test vettori zlib |
| `sim/src/zip.rs` | ZIP lettura (stored/deflate) e scrittura stored | inflate | fatto |
| `sim/src/xml.rs` | tokenizer XML | - | fatto |
| `sim/src/office.rs` | testo DOCX/ODT/ODS/XLSX; `read_text` | zip, xml | fatto |
| `sim/src/pdf.rs` | testo PDF di base | inflate, office (Sink) | fatto, base |
| `sim/src/officewrite.rs` | DOCX/ODT minimi | zip | fatto, da verificare in Word/LibreOffice |
| Pages (IWA) | - | - | da fare |

Test: `cd sim && cargo test --test office` (14 test).
