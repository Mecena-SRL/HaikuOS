# Roadmap del file system

Stato: proposta. Data fonti: 2026-11-25. Ogni ipotesi non misurata è marcata **da verificare**.
Riferimenti: `docs/filesystem-review.md`, `gba/src/fs.rs`, `sim/src/snapshot.rs`.

## 1. Obiettivo

"Il più avanzato possibile" non ha una definizione fissa. Qui il valore è la somma di funzioni reali, misurate e sicure, su due piattaforme:

- **GBA**: RAM e supporto (SRAM/flash) piccoli, nessuna CPU per checksum pesanti.
- **Macchina potente** (ARM64/AMD64): più RAM e CPU, dischi e SSD veri.

## 2. Fonti consultate

Verificate sul testo ufficiale:

- Btrfs, stato delle funzioni (kernel/btrfs-progs): https://raw.githubusercontent.com/kdave/btrfs-progs/master/Documentation/Status.rst
- F2FS, documentazione kernel: https://raw.githubusercontent.com/torvalds/linux/master/Documentation/filesystems/f2fs.rst
- OpenZFS, concetti (snapshot, checksum): https://raw.githubusercontent.com/openzfs/zfs/master/man/man7/zfsconcepts.7
- OpenZFS, feature dei pool (block cloning, raidz expansion, fast dedup, device removal): https://raw.githubusercontent.com/openzfs/zfs/master/man/man7/zpool-features.7
- LittleFS, design (resilienza a interruzioni di alimentazione, wear leveling): https://raw.githubusercontent.com/littlefs-project/littlefs/master/DESIGN.md

Solo da risultati di ricerca (testo non letto, **da verificare**):

- bcachefs, documentazione kernel e erasure coding: https://docs.kernel.org/filesystems/bcachefs/index.html
- APFS, guida Apple alla sicurezza: https://support.apple.com/guide/security/seca6147599e/web
- ReFS, integrity streams e block cloning: https://learn.microsoft.com/windows-server/storage/refs/refs-overview
- WAFL, paper NetApp (1994, copy-on-write con snapshot): https://static.usenix.org/publications/library/proceedings/sf94/full_papers/hitz.a

Non raggiungibili dal sandbox: docs.kernel.org e support.apple.com (rete bloccata). Da leggere prima di implementare.

## 3. Funzioni e stato dell'arte

| Funzione | Chi la fa (fonte) | Nota per noi |
|---|---|---|
| Copy-on-write (mai sovrascrivere) | ZFS, Btrfs, APFS, WAFL, LittleFS | Base dell'atomicità. Abbiamo già la forma minima in `fs.rs`. |
| Snapshot istantanei | ZFS: "creati rapidamente, inizialmente nessuno spazio extra" (zfsconcepts). Btrfs: OK (Status.rst). | Costo = copia della tabella, non dei dati. Prototipo fatto (vedi §6). |
| Checksum su dati **e** metadati | ZFS (checksum end-to-end), Btrfs: checksum OK (Status.rst), ReFS integrity streams | Oggi copriamo dati e metadati delle voci (`fs.rs`); puntatori `next` coperti da checksum per blocco, verificati in lettura, scrittura e rimozione. |
| Auto-riparazione da copia valida | Btrfs: OK su DUP, RAID1, RAID10, RAID56 | Richiede ridondanza. |
| Scrub (verifica periodica) | Btrfs: OK; Scrub + RAID56: "mostly OK" | RAID5/6 non è maturo ovunque: evitare in v1. |
| Compressione | Btrfs OK (Status.rst), ZFS (feature in pool) | Riusare LZSS già in `sim/src/lzss.rs`. **Da verificare** rapporto e CPU su ARM7. |
| Cifratura | ZFS (feature `encryption`), APFS | Riusare Ascon-AEAD in `sim/src/slot.rs`. **Da verificare** AAD per file. |
| Block cloning / reflink | ZFS `block_cloning`, ReFS block cloning, Btrfs reflink (Status.rst: "mostly OK") | Copia costosa evitata. Richiede tabella di riferimenti. |
| Deduplica | Btrfs out-of-band "mostly OK", ZFS `fast_dedup` | Costo RAM alto per tabella hash. Bassa priorità. |
| Ridimensionamento online | Btrfs resize OK; ZFS `raidz_expansion` (feature) | Utile solo con dischi veri. |
| Rimozione device / sostituzione | ZFS `device_removal`, Btrfs device replace "mostly OK" | Solo macchina potente. |
| Gestione SSD (TRIM, scrittura distribuita) | F2FS: discard in tempo reale o differito; separazione hot/cold su più log; LittleFS: wear leveling dinamico | Su GBA: wear leveling solo se il supporto ha endurance limitata. **Da verificare** con dato del produttore. |
| Resilienza a caduta di alimentazione | LittleFS (metadata pair, due regioni alternate) | Già nel nostro `persist.rs`. |

## 4. Piattaforma

| Funzione | GBA | Macchina potente |
|---|---|---|
| Copy-on-write + atomicità | Sì (già) | Sì |
| Checksum metadati | Sì, poco costo (32 bit per file) | Sì |
| Checksum puntatori | Da verificare (budget EWRAM) | Sì |
| Snapshot leggeri | Solo se la tabella entra in RAM | Sì |
| Compressione | Forse, per documenti e giochi | Sì |
| Cifratura | Sì, se il costo CPU regge. **Da verificare** | Sì |
| RAID / ridondanza | No | Sì, mirror prima di RAID5/6 |
| Wear leveling | Solo se endurance limitata | Gestito dall'SSD |
| Dedup, block cloning | No | Più avanti |

## 5. Priorità (costo / beneficio)

| # | Funzione | Costo stimato | Beneficio | Piattaforma | Dipende da |
|---|---|---|---|---|---|
| 1 | Checksum su `Entry` (nome, owner, testa, lunghezza) | ~30 righe, 4 byte per file | Alto: corruzione silenziosa dei permessi | Entrambe | nessuno |
| 2 | Snapshot leggeri (copy-on-write a blocchi) | Prototipo fatto nel sim; ~1 blocco per file modificato | Alto: rollback, base per backup | Entrambe, il GBA con limiti | nessuno |
| 3 | Checksum puntatori `next` | 4 byte per blocco | Medio | Entrambe | 1 |
| 4 | Compressione (LZSS) per file | Riuso del codice in sim | Medio | Entrambe | nessuno |
| 5 | Cifratura per file (Ascon) | Riuso del codice in sim | Alto per sicurezza | Entrambe | 1 |
| 6 | Contatore cancellazioni per blocco | 1 byte per blocco | Solo con supporto a endurance limitata | GBA | verificare endurance |
| 7 | Send/receive di snapshot (incrementale) | Medio-alto | Medio (backup) | Macchina potente | 2 |
| 8 | Mirror (2 copie + auto-riparazione) | Medio | Alto su disco | Macchina potente | 1, 2 |
| 9 | Block cloning / reflink | Medio | Medio | Macchina potente | 2 |
| 10 | RAID5/6, dedup, resize online | Alto | Basso in v1 | Macchina potente | 8 |

## 6. Primi passi (fatti in questa PR)

- `sim/src/snapshot.rs`: store a blocchi con riferimenti. Snapshot = copia della tabella file -> blocco. Scrittura = blocco nuovo. Rollback e drop con conteggio dei riferimenti.
- `sim/tests/snapshot.rs`: 9 test. Costo zero in blocchi allo snapshot; un blocco per overwrite; drop libera i blocchi; rollback recupera file cancellati; limiti (64 blocchi, 4 snapshot, 64 byte per file) **da verificare**.
- Snapshot: `verify()` controlla i checksum di tabella viva e snapshot. Il rollback verifica lo snapshot prima di toccare i riferimenti: se è alterato, errore `Corrupt` e stato invariato. Test di manomissione.
- Checksum sui metadati (nome, blocco, lunghezza) per ogni voce, verificato in `read_checked`; test di manomissione. È il passo 1 della priorità, portato nel sim.
- Verifica di integrità dell'invio (`sim/src/snapshot.rs`): `diff` include un'impronta dello stato di arrivo; dopo `apply` la replica ricalcola l'impronta e segnala `Corrupt` se non coincide. La replica deve essere nello stato di partenza. `apply` lavora su una copia di lavoro: in caso di errore la replica resta com'era. Costo: la copia dell'intero store (accettabile nel simulatore, da rivedere su GBA).
- Chiavi per replica (`sim/src/keys.rs`): ogni replica ha una chiave derivata dalla chiave madre e dall'epoca. Rotazione manuale con `rotate_key`: ricifra tutti i blocchi in uso, ogni riscrittura conta per l'usura. Schema di derivazione segnaposto: da verificare con una KDF dedicata. 4 test in `tests/chiavi.rs`.
- Invio e ricezione incrementale simulati (`sim/src/snapshot.rs`, priorità 7): `diff(from, to)` produce i nomi cambiati e quelli cancellati tra due snapshot; `apply` li applica a una replica. Il contenuto viaggia cifrato con lo stesso schema dei blocchi (LZSS, Ascon, nonce da contatore, AAD distinta). `apply` autentica tutto prima di scrivere. Chiave condivisa tra le repliche: da verificare come distribuirla. 3 test.
- Deduplica simulata (`sim/src/snapshot.rs`, priorità 10): una scrittura con contenuto già presente in un blocco vivo riusa quel blocco, dopo verifica del contenuto (non solo dell'impronta). Nessuna riscrittura, quindi nessun consumo di endurance. 3 test più i contatori `dedup_hits`.
- Block cloning simulato (`sim/src/snapshot.rs`, priorità 9): `clone_file` crea un nome che condivide il blocco, senza copiare dati. Una scrittura su uno dei due alloca un blocco nuovo; il blocco si libera solo con l'ultimo nome. 4 test.
- Mirror simulato (`sim/src/snapshot.rs`, priorità 8): ogni blocco ha due copie. La lettura usa la copia valida; `scrub()` ripara quella danneggiata dall'altra e segnala se entrambe sono perse. Costo: il doppio dei blocchi fisici. Ancora niente RAID (decisione 3).
- Contatore di riscritture per blocco (`sim/src/snapshot.rs`): l'allocazione sceglie il blocco libero meno usato (wear leveling) e rifiuta i blocchi oltre l'endurance (`Worn`). Endurance segnaposto 100 000, da verificare con il dato del produttore. Protezione a costo basso, anche senza prova di endurance limitata.
- Cifratura nello snapshot (`sim/src/snapshot.rs`): ogni blocco è LZSS poi Ascon-AEAD via `slot.rs`. L'AAD lega il dato al suo indice di blocco, quindi uno scambio di blocchi è rilevato. Chiave in `Store::new`, nonce da un contatore. Blocco 128 byte, testo in chiaro max 256 (da verificare). Byte alterato in un blocco rilevato in lettura.
- Compressione nello snapshot (ora dentro la cifratura) (`sim/src/snapshot.rs`): LZSS per file, usato solo se sta in un blocco e riduce la dimensione; altrimenti in chiaro. 
- `gba/src/fs.rs`: checksum dei metadati per ogni voce (nome, flag, owner, testa, lunghezza), verificato in `entry()`; checksum dei puntatori `next` per blocco, verificati lungo la catena; test di manomissione. 33 test host verdi, build GBA release ok.

Limiti dichiarati: un file occupa un solo blocco (64 byte). Overwrite richiede un blocco libero, anche se il file è piccolo. Non c'è persistenza su disco.

## 7. Dipendenze e stime

- Passo 1 (checksum `Entry`): nessuna dipendenza. Stima: una PR piccola, con test di corruzione.
- Passo 2 (snapshot su GBA): serve misurare la RAM libera (EWRAM) e la dimensione della tabella. **Da verificare** con `docs/feasibility-gba.md`.
- Passi 7-9: dipendono dall'hardware reale. Stima non fattibile oggi.

## 8. Da verificare

- Endurance reale della SRAM/flash della cartuccia.
- Costo CPU di checksum e compressione su ARM7TDMI (metodo: vedi PR #11).
- Testi ufficiali di bcachefs, APFS, ReFS, WAFL: letti solo dai risultati di ricerca.
- Il fatto che RAID5/6 su Btrfs sia "mostly OK" va rifatto dopo ogni nuova versione del kernel.

## 9. Decisioni

Prese dal coordinatore (scelte tecniche, 2026-11-25):

1. **Copy-on-write con snapshot** come principio di base.
2. **Checksum su dati e metadati, sempre.** Sul GBA il costo si misura e si adatta con il profilo.
3. **Ridondanza dentro l'OS rimandata** alla macchina potente, fase successiva. Per ora nessun RAID.

Da confermare con Ivan (filosofia): nessuna.

Tappe successive: checksum sui metadati (`Entry`) e snapshot, nel simulatore.
