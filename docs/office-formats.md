# Formati Office e OpenDocument

> Il codice sta in `office/` (vedi [office-suite.md](office-suite.md)). Questo file è l'indice dei formati.
> Ogni misura non provata è **da verificare**.

| Formato | Lettura | Scrittura | Dove |
|---|---|---|---|
| DOCX | testo | testo minimo | `office/src/formats.rs` |
| ODT | testo | testo minimo | `office/src/formats.rs` |
| ODS | sì | no | `office/src/ods.rs` |
| XLSX | primo foglio, testo | no | `office/src/sheet.rs` |
| PDF | testo di base | no | `office/src/pdf.rs` |
| Pages (`.pages`) | no | no | da fare: IWA (Snappy + protobuf). **Da verificare** |

File di prova: `office/tests/fixtures/`. Nessun file creato con Word, LibreOffice o Pages: compatibilità reale **da verificare**.
