# Compatibilità file system e partizioni

Stato: proposta. Data: 2026-11-25. Ipotesi non lette sul testo ufficiale: **da verificare**.
In questa PR nessuna specifica è stata scaricata: i link sotto vanno letti prima di implementare.

## 1. Principio

- Formattazione, partizionamento e cifratura sono **programmi**, non nucleo.
- Il nucleo espone solo blocchi con checksum (`sim/src/storage.rs`). I formati vivono in un programma utente separato.
- Compatibilità = il programma legge e scrive i formati standard di Mac, Windows e Linux. Il formato nativo di HaikuOS resta separato.

## 2. Ordine di lavoro

1. Lettura sola lettura: MBR, GPT, FAT32, exFAT, ext4.
2. Scrittura: FAT32 ed exFAT, poi ext4 (journal e checksum dei metadati).
3. NTFS e APFS, in sola lettura prima. HFS+ solo in lettura, per dischi vecchi.

## 3. Formati

| Formato | Dove | Fonte ufficiale (da leggere) | Leggere | Scrivere | Rischi | Licenza / brevetti |
|---|---|---|---|---|---|---|
| MBR | BIOS legacy, protettiva per GPT | UEFI 2.10, sez. 5.2 (uefi.org/specifications) | 4 voci primarie | Solo per la MBR protettiva | Limite 2 TiB; voce 0xEE da non confondere | Libera |
| GPT | UEFI, Mac Intel, Windows, Linux | UEFI 2.10, sez. 5.3 | Intestazione e array di voci, CRC32 | Scrivere **entrambe** le intestazioni (primaria e backup) con i CRC | Una sola copia aggiornata rende il disco illeggibile per alcuni sistemi | Libera |
| FAT32 | Chiavette, EFI | Microsoft, "FAT specification" (fatgen103) | BPB, FAT, directory, nomi lunghi (LFN) | Due copie della FAT, FSInfo | Nessun journal: spegnimento durante scrittura = perdita | LFN: brevetto Microsoft, stato **da verificare** |
| exFAT | Chiavette, SD, Mac e Windows | Microsoft, specifica exFAT (Open Specifications) | Bitmap, checksum delle voci di directory | Come FAT, con checksum | Nessun journal | Microsoft chiede licenza per exFAT: condizioni **da verificare** |
| ext4 | Linux | docs.kernel.org/filesystems/ext4 | Superblock, gruppi, inode, extent, journal (JBD2) | Journal, metadata_csum | Lettura senza replay del journal = dati incoerenti | Specifica libera; implementazione GPL |
| NTFS | Windows | Linux-NTFS (non ufficiale); [MS-FSCC] Microsoft | MFT, attributi, runlist | Sconsigliato prima di test estesi | $LogFile, bit "sporco", Fast Startup | Brevetti: **da verificare** |
| APFS | Mac moderni | Apple, "Apple File System Reference" (PDF su developer.apple.com) | Container, volumi, B-tree, snapshot | Più avanti | Cifratura per volume (keybag), snapshot | Documentazione Apple, condizioni **da verificare** |
| HFS+ | Mac vecchi | Apple TN1150 | B-tree, catalogo, journal | No | Journal | Documentazione Apple, condizioni **da verificare** |

## 4. Dove vive il codice

- **Programma disco** (nome da scegliere), in user space. Un modulo per formato. Riceve blocchi dal driver storage.
- **Nucleo**: nessun formato. Solo blocchi, checksum e permessi.
- **Simulatore**: `sim/src/partition.rs`, lettura GPT e MBR in sola lettura.

## 5. Cosa è verificabile senza hardware

- Tabelle GPT e MBR: sì. Immagini generate dal codice nei test (`sim/tests/particioni.rs`, 11 test).
- FAT32, exFAT, ext4, NTFS, APFS: serve un'immagine di riferimento creata da un altro sistema. Nel repo non entrano file esterni. Il confronto con Mac e Windows richiede una VM o un disco: **da verificare**.
- Cifratura APFS, dischi reali e prestazioni: no.

## 6. Regole di sicurezza

- Nessuna scrittura su disco senza conferma esplicita (come nella PR #44).
- Scrivere solo dopo lettura riuscita e verifica dei CRC o dei checksum.
- Test sempre su immagini, poi su un disco di prova.

## 7. Da verificare

- Testi ufficiali di tutte le specifiche: non letti in questa PR.
- Licenze e brevetti: FAT LFN, exFAT, NTFS, APFS, HFS+.
- Limiti del prototipo GPT: massimo 128 voci da 128 byte, settori da 512 byte (niente 4Kn), intestazione di backup non letta.
- Il programma disco ha nome provvisorio, deciso da Ivan.

## 8. Stato FAT32 ed exFAT (aggiornato)

- FAT32: lettore in sola lettura in `sim/src/fat32.rs`, 8 test in `sim/tests/fat32.rs`. Offset e regole verificati sull'implementazione Linux (fs/fat, GPL), non sul testo Microsoft: il sandbox non ha raggiunto `download.microsoft.com`. **Da verificare** sul fatgen103.
- exFAT: lettore in sola lettura in `sim/src/exfat.rs`, 9 test in `sim/tests/exfat.rs`. Offset dall'header Linux (fs/exfat/exfat_raw.h), checksum dell'insieme di voci da fs/exfat/dir.c. Specifica Microsoft non raggiunta dal sandbox. **Da verificare**.
- Limiti dei lettori: un solo FAT (niente TexFAT), nessun controllo del nome hash exFAT, niente tabella upcase, niente scrittura.
- Limiti del lettore FAT32: nomi solo dalla voce corta se il checksum LFN non combacia; niente bit di minuscole del nome corto; niente scrittura.
