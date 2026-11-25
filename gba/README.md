# HaikuOS su GBA: prima ROM

Prima ROM: monolite, agente di allocazione e slot. Mostra lo stato a schermo. Nessun bootloader separato.

## Build

Serve Rust nightly con `rust-src` (già in `rust-toolchain.toml`). Target `thumbv4t-none-eabi`, con `build-std`.

```sh
cd gba
cargo build --release
/usr/bin/llvm-objcopy -O binary target/thumbv4t-none-eabi/release/haiku-gba haiku-gba.gba
```

## Cosa mostra la ROM

- Barra in alto: memoria del pool (verde monolite, blu agente, arancio slot assegnati, grigio scuro libero).
- Quattro quadrati: arancio = slot assegnato, grigio = libero. Sequenza di boot: 3 concessi, il 4° negato, revoca, riassegnazione.
- Tre quadretti in basso: monolite (verde), agente (blu), self-test del file system (verde = ok).

## File system (`src/fs.rs`)

- Store a 128 blocchi da 64 byte (8 KB). Nomi e contenuti separati. Max 16 file, nomi da 12 byte.
- Checksum FNV-1a su ogni blocco, verificato in lettura.
- Scrittura atomica: blocchi nuovi prima, cambio di testa poi, liberazione del vecchio. Se manca spazio, il file resta uguale.
- Permessi: solo l'owner. Quota: la scrittura non supera i byte concessi dall'agente.
- Test host: `cd gba && rustc --edition 2021 --test src/fs.rs -o /tmp/fs_test && /tmp/fs_test` (5 test, tutti ok).
- Non ancora: directory (solo lista piatta), persistenza tra riavvii (serve la memoria di salvataggio della cartuccia, da decidere).

## Verificato

- Compila con nightly (`thumbv4t-none-eabi`, `build-std`). Binario ~1,1 KB.
- Header ROM corretto (branch d'ingresso, titolo, byte fisso 0x96). Entry a 0x08000000.
- Avvio in mGBA 0.10.2 (software renderer, Xvfb): la schermata si vede come descritta.

## Da verificare / non fatto

- **Hardware reale:** il logo Nintendo (0x04-0x9F) e il checksum (0xBD) non sono impostati. mGBA li ignora; il BIOS reale no.
- Logica del monolite: porting manuale da `sim/src/lib.rs`. Nessun test sul target: i test di `sim/` coprono la versione host.
- Dimensione reale del codice e uso reale di RAM: non misurati oltre al binario.
- Messaggi (filtro allowlist) non portati: fuori dal primo passo.
- Allocatore dentro gli slot: non ancora.
- Quanta EWRAM resta davvero libera: da verificare.
