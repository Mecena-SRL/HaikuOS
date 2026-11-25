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
- Tre quadretti in basso: monolite (verde), agente (blu), persistenza (verde = ok).
- Riga blu in basso: un quadretto per ogni avvio salvato (il contatore sopravvive al riavvio).

## File system (`src/fs.rs`)

- Store a 128 blocchi da 64 byte (8 KB). Nomi e contenuti separati. Max 16 file, nomi da 12 byte.
- Checksum FNV-1a su ogni blocco, verificato in lettura.
- Scrittura atomica: blocchi nuovi prima, cambio di testa poi, liberazione del vecchio. Se manca spazio, il file resta uguale.
- Permessi: solo l'owner. Quota: la scrittura non supera i byte concessi dall'agente.
- Test host: `cd gba && rustc --edition 2021 --test src/fs.rs -o /tmp/fs_test && /tmp/fs_test` (5 test, tutti ok).
- Non ancora: directory (solo lista piatta).

## Verificato

- Compila con nightly (`thumbv4t-none-eabi`, `build-std`). Binario ~1,1 KB.
- Header ROM corretto (branch d'ingresso, titolo, byte fisso 0x96). Entry a 0x08000000.
- Avvio in mGBA 0.10.2 (software renderer, Xvfb): la schermata si vede come descritta.

## Non fatto (vecchio elenco)

- **Hardware reale:** il logo Nintendo (0x04-0x9F) e il checksum (0xBD) non sono impostati. mGBA li ignora; il BIOS reale no.
- Logica del monolite: porting manuale da `sim/src/lib.rs`. Nessun test sul target: i test di `sim/` coprono la versione host.
- Dimensione reale del codice e uso reale di RAM: non misurati oltre al binario.
- Messaggi (filtro allowlist) non portati: fuori dal primo passo.
- Allocatore dentro gli slot: non ancora.
- Quanta EWRAM resta davvero libera: da verificare.

## Persistenza (`src/persist.rs`)

- Supporto: il livello sottile `Medium`. Sul GBA è la SRAM della cartuccia da 32 KB (`src/sram.rs`, tag `SRAM_V113` per mGBA). Sulla macchina potente sarà un file.
- Journal con checksum FNV-1a su ogni record. Due regioni di log alternate e due slot di intestazione. Quando il log è pieno, lo stato si riscrive nella regione libera (compattazione), poi cambia la testa.
- Un byte 0 dopo la coda del log: il replay si ferma lì e non legge record vecchi.
- Salvataggio incrementale: solo i file cambiati dall'ultimo salvataggio, con più modifiche dello stesso file coalescenti in una scrittura.
- Regole di salvataggio (profilo del supporto): salva a soglia di modifiche (`units`) o dopo `max_frames`, solo se c'è almeno una modifica. Limite di `max_writes_per_min` salvataggi per minuto.
- Profilo GBA: soglia 64, tempo 1800 frame (30 s), 2 salvataggi al minuto. Profilo macchina potente: da definire.
- Test host: 17 test, inclusi limiti di salvataggi e assenza di salvataggi senza modifiche. `cd gba && rustc --edition 2021 --test host/test_root.rs -o /tmp/gba_t && /tmp/gba_t`.
- Verificato in mGBA: il contatore di avvii sale da 1 a 2 tra due esecuzioni (il `.sav` contiene i dati).

## Da verificare

- Cicli di scrittura della SRAM e della flash GBA: non misurati. Il profilo sceglie frequenza bassa per questo motivo.
- Su hardware reale: non testato. SRAM e tempi di scrittura sono da misurare.
- Stack: il file system e il persistente stanno nell'IWRAM da 32 KB. Non misurato oltre a quello che mGBA mostra.
