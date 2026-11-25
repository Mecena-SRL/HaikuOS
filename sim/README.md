# Simulatore di allocazione (host)

Prototipo sull'host: monolite, agente di allocazione e filtro messaggi. Nessun hardware né bootloader.

## Come eseguire i test

Serve Rust (cargo).

```sh
cd sim
cargo test
```

## Cosa verificano i test (`tests/limiti.rs`)

- massimo 4 programmi registrati;
- richieste oltre lo slot (o zero, o programmi sconosciuti) negate;
- uno slot sempre riservato: al massimo 3 quote concesse insieme;
- revoca che libera lo slot;
- agente non fidato (quota enorme o maggiore della richiesta) negato dal monolite;
- messaggi fuori allowlist, oltre la dimensione fissa o senza slot scartati;
- un messaggio non concede né modifica diritti.

## Snapshot copy-on-write (`src/snapshot.rs`, `tests/snapshot.rs`)

- Snapshot = copia della tabella file -> blocco, nessun dato copiato; overwrite usa un blocco nuovo.
- Test: rollback, drop che libera blocchi, limiti (blocchi, snapshot, dimensione). Roadmap: `docs/filesystem-roadmap.md`.

## Scelte

- Linguaggio: Rust, `std` per la simulazione sull'host. La logica usa solo `Vec` piccoli e costanti, quindi è portabile a `no_std` + `alloc`. **Da verificare** con Ivan.
- Limiti: costanti in `src/lib.rs`, valori di `docs/architecture.md` sezione 6. Tutti **da verificare**.

## Da verificare

- Lettura di "massimo 4 programmi" e "uno slot sempre libero": qui 4 slot, 3 concedibili.
- Dimensione massima messaggio (64 byte).
- Memoria totale: 288 KB, con 176 KB concedibili (monolite 64 + agente 16 + 3 slot da 32).

## Memoria compressa e cifrata (`src/lzss.rs`, `src/slot.rs`)

- LZSS a finestra 256 byte, poi Ascon-AEAD128 con AAD che lega il blocco allo slot.
- Test in `tests/memoria.rs`: `cargo test`.
- Misure e limiti: `docs/memory-security.md`, sezione 7. **Da verificare** su GBA.

## Tabelle delle partizioni (`src/partition.rs`, `tests/particioni.rs`)

- Lettura in sola lettura di MBR e GPT da un'immagine in memoria. Verifica i CRC32 e i limiti del disco.
- Scelte e limiti: `docs/compatibility.md`. Non legge l'intestazione di backup GPT. **Da verificare**.

## Gestore dei font (`src/fonts.rs`, `tests/fonts.rs`)

- Registro, attivazione (max 4), font di sistema non disattivabile, collezioni (max 8, 8 membri ciascuna), anteprima.
- Solo host, nessun parser TTF/OTF. Design e limiti: `docs/font-manager.md`. **Da verificare** su GBA.
