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

## Scelte

- Linguaggio: Rust, `std` per la simulazione sull'host. La logica usa solo `Vec` piccoli e costanti, quindi è portabile a `no_std` + `alloc`. **Da verificare** con Ivan.
- Limiti: costanti in `src/lib.rs`, valori di `docs/architecture.md` sezione 6. Tutti **da verificare**.

## Da verificare

- Lettura di "massimo 4 programmi" e "uno slot sempre libero": qui 4 slot, 3 concedibili.
- Dimensione massima messaggio (64 byte).
- Memoria totale: 288 KB, con 176 KB concedibili (monolite 64 + agente 16 + 3 slot da 32).
