# Ricompilatore (C -> Rust nativo)

Non è un emulatore né un layer tipo Rosetta/Proton. Il software open source viene tradotto in Rust `no_std` e compilato per HaikuOS.

## Pipeline

1. **Sorgente**: un file C pubblico (o più file) da tradurre.
2. **Traduzione**: un LLM via API (DeepSeek di default, endpoint compatibile OpenAI). Il prompt impone `no_std`, niente alloc, niente crate esterni, firme pubbliche invariate.
3. **Verifica**: il Rust generato va in una crate di prova e viene eseguito `cargo test`, con i test di specifica scritti a mano. Accettato solo se compila e tutti i test passano.
4. **Ritento**: fino a 3 tentativi; l'errore del compilatore torna al modello.

Il codice generato non entra mai nel repo senza passare il punto 3.

## Uso (v0)

```
cd tools/recompiler && cargo build
DEEPSEEK_API_KEY=... target/debug/recompiler <sorgente.c> <test.rs> <cartella_out>
```

Offline (test): `RECOMP_MOCK=<file_risposta> ...` usa una risposta fissa al posto dell'API.

Test del percorso HTTP senza rete: `fixtures/fake_api.py` (server locale su 127.0.0.1:8765, chiave di test `testkey`) con `RECOMP_URL=http://127.0.0.1:8765/chat/completions DEEPSEEK_API_KEY=testkey`. Verificato: OK al tentativo 1.

Fixture: `tools/recompiler/fixtures/` (djb2: C, test, risposta buona e risposta errata).

## Host supportati

Il tool è pensato per girare su **Fedora KDE** e **macOS**. Richiede solo `cargo` (rustup) e `curl`, presenti su entrambi. Nessun percorso specifico di sistema.

- Fedora: `sudo dnf install curl` e `rustup` (una volta).
- macOS: `curl` è già presente; `rustup` da rustup.rs.
- **Da verificare** su entrambi: il test `cargo test` delle fixture non è ancora stato eseguito fuori da Linux.

## Stato

- Verificato: pipeline con risposta fissa. Risposta buona: OK al primo tentativo. Risposta errata (overflow non gestito): rifiutata dopo 3 tentativi, exit 1.
- **Da verificare**: traduzione reale con DeepSeek. L'ambiente di sviluppo non raggiunge `api.deepseek.com`.
- Secondo caso: `adler32.c` (core di zlib, licenza zlib), test con valori di riferimento calcolati da `zlib` Python. Traduzione in `fixtures/adler32_translated.rs`: **scritta a mano da un agente, non dall'LLM**, passata dalla pipeline (mock): OK al tentativo 1.
- Terzo caso: `sds_ll2str.c` (da SDSLib, licenza BSD-2), test su i64 incluso MIN/MAX. Traduzione `fixtures/sds_ll2str_translated.rs`, anche questa **scritta a mano**, OK al tentativo 1.
- **Da fare**: traduzione reale con DeepSeek di questi sorgenti e di altri repo pubblici.

## Fuori scope adesso

Layer di compatibilità (Linux/Unix): fase successiva, vedi `docs/recompiler-compat-layer.md`.
