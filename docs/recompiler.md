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

Fixture: `tools/recompiler/fixtures/` (djb2: C, test, risposta buona e risposta errata).

## Stato

- Verificato: pipeline con risposta fissa. Risposta buona: OK al primo tentativo. Risposta errata (overflow non gestito): rifiutata dopo 3 tentativi, exit 1.
- **Da verificare**: traduzione reale con DeepSeek. L'ambiente di sviluppo non raggiunge `api.deepseek.com`.
- **Da fare**: sorgenti open source pubblici reali (utility, giochi semplici), con test di specifica tratti dal sorgente stesso.

## Fuori scope adesso

Layer di compatibilità (Rosetta-like, Linux/Unix): fase successiva.
