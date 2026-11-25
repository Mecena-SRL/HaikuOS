# Revisione del file system

Stato: revisione documentale + una modifica piccola in `gba/src/fs.rs`. Le ipotesi non misurate sono marcate **da verificare**.

## 1. Stato attuale

- `gba/src/fs.rs`: store in RAM a blocchi fissi (128 × 64 byte), nomi e contenuti separati, catena di blocchi per file, checksum FNV-1a per blocco, scrittura atomica (nuovi blocchi prima, cambio di testa poi), directory, permessi per owner, quota per scrittura.
- `gba/src/persist.rs`: journal con checksum su un supporto a blocchi (SRAM sul GBA, file sulla macchina potente), due regioni di log alternate, compattazione quando il log è pieno.
- Test host: 28 verdi (`rustc --edition 2021 --test host/test_root.rs`). Build GBA release: ok.

## 2. Cosa è buono

- Scrittura atomica senza journal sul file system in RAM: i blocchi nuovi si preparano prima, la testa cambia alla fine. Stesso principio del copy-on-write, in forma minima.
- Checksum per blocco e rilevamento della corruzione alla lettura (test `corruption_detected_and_remove`).
- Persistenza a log con due regioni alternate: la compattazione riscrive in una regione libera, non sovrascrive i dati validi. È la stessa idea di LittleFS (metadata pair con scambio) e di sequential-storage (log circolare per NOR).
- Nessuna allocazione dinamica: adatto a `no_std` e a memoria fissa.

## 3. Cosa manca

| Lacuna | Rischio | Nota |
|---|---|---|
| Checksum solo sui dati, non sui metadati (`Entry`: nome, owner, testa, lunghezza) | Corruzione silenziosa di nomi/permessi | ZFS e btrfs proteggono anche i metadati con checksum. **Da verificare** quanto conti in RAM senza ECC. |
| Nessun checksum sui puntatori `next` | Catena corrotta non rilevata prima della lettura | Stesso punto. |
| Allocazione dal primo blocco libero (ora next-fit, vedi §5) | Usura concentrata su pochi blocchi, se lo store viene mirrorato su flash | Vale solo per un supporto con cancellazioni (flash/SRAM con endurance limitata). In RAM non c'è usura. |
| Nessun limite di cicli di scrittura per blocco | Nessun controllo di usura | Su SRAM/flash del cartuccia: endurance **da verificare** (dato del produttore). |
| Riavvio durante un salvataggio: copertura solo del caso "log troncato" | Perdita dell'ultimo salvataggio | Il replay si ferma al primo record non valido (test `log troncato`). Altri casi di scrittura parziale: **da verificare**. |

## 4. Stato dell'arte (sintesi)

Fonti da verificare (la ricerca ha restituito pagine, non il testo completo):

- LittleFS, design (metadata pair, copy-on-write, wear leveling dinamico, resilienza a interruzioni): https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md
- Btrfs, design (checksum CRC32C su dati e metadati, copy-on-write): https://btrfs.readthedocs.io/en/latest/dev/dev-btrfs-design.html
- ZFS, Merkle tree dei checksum nel puntatore padre: https://web.mit.edu/6.1800/www/papers/zfs.pdf
- sequential-storage (log per NOR flash, wear leveling): https://docs.rs/sequential-storage
- ekv (key-value per flash embedded, Rust): https://docs.rs/crate/ekv/latest
- F2FS e UBIFS (log-structured per NAND, separazione hot/cold, wear leveling nel livello UBI): https://en.wikipedia.org/wiki/Flash_file_system e https://arxiv.org/pdf/1208.6390

Sintesi:
- **Flash / memorie piccole**: log o copy-on-write con metadata a coppie e wear leveling dinamico (LittleFS, sequential-storage). Il nostro journal è già di questo tipo.
- **Sistemi potenti**: checksum end-to-end su dati e metadati (ZFS, btrfs), copy-on-write con snapshot. Costo: più CPU e RAM per i checksum, più scritture per i puntatori.
- **Da evitare su GBA**: journal completo su blocchi (ext4, btrfs) — troppa RAM e troppe scritture.

## 5. Proposte

| # | Proposta | Costo stimato | Stato |
|---|---|---|---|
| 1 | Allocazione next-fit nello store (fatto in questa PR) | ~10 righe, nessuna RAM in più (un `usize`) | Fatto, test `writes_spread_over_blocks` |
| 2 | Checksum su `Entry` (nome, owner, testa, lunghezza) verificato a ogni accesso | ~30 righe, 1 `u32` per file (16 file = 64 byte) | Da fare |
| 3 | Checksum sui puntatori `next` dentro il blocco, in parallelo ai dati | 4 byte per blocco (128 blocchi = 512 byte) | Da verificare sul budget RAM (EWRAM libera, vedi `docs/feasibility-gba.md`) |
| 4 | Contatore di cancellazioni per blocco nel supporto persistente | 1 byte per blocco; soglia di compattazione | Da verificare: serve solo se il supporto ha endurance limitata |
| 5 | Copy-on-write con snapshot | Alto: riscrittura del modello di blocchi | Non proposto per GBA; valutare sulla macchina potente |

## 6. Cosa non ho verificato

- Endurance reale della SRAM/flash della cartuccia: non misurata.
- Costo dei checksum su ARM7TDMI: non misurato (solo ciclo di timer, vedi PR #11 per il metodo).
- Effetto della next-fit sull'usura reale: non misurato, è un'ipotesi di design.
