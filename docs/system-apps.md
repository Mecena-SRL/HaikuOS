# App di sistema (sysapps)

> Versione logica. Tutto ciò che è segnato **da verificare** non è stato provato su hardware.

## Cosa c'è

Crate `sysapps` (Rust, `no_std`, senza heap). Test sul computer host: `cargo test -p sysapps`.

| App | Lanciatore | Terminale | Stato |
|---|---|---|---|
| Browser | "Browser" | `browse <pagina>` stampa la pagina | logica e cronologia testate |
| Editor script | "Editor script" | `run <script>` esegue il file | buffer e esecuzione testati |

Comandi del terminale: `help`, `apps`, `echo <testo>`, `browse <pagina>`, `run <script>`, `# commento`.
`run` non può essere usato dentro uno script (nessuna ricorsione).

## Formato pagina

```
# Titolo
=> nome_pagina Etichetta del collegamento
testo normale
```

## Limiti

- **Rete:** nessuna. Gli URL `http://` e `https://` danno `Offline` finché il net stack non è collegato.
- **Cronologia browser:** 8 voci (capacità a compile time, la più vecchia viene scartata).
- **Buffer editor:** 32 righe × 64 colonne nel test. Solo ASCII stampabile. **Da verificare** contro la RAM del GBA.
- **Lanciatore:** esiste solo il registro `APPS`. L'interfaccia grafica non è ancora scritta. **Da verificare** su schermo.
- **Rendering:** nessun disegno a schermo. **Da verificare** su hardware.
- **Sottoinsieme shell:** nessuna variabile, nessun pipe, nessun file system scrivibile.

## Prossimi passi

1. Collegare `Store` al file system reale (**da verificare** dove stanno le pagine).
2. Disegno del browser e dell'editor nel lanciatore.
3. Net stack, per aprire URL veri.

## Vault: punti di sicurezza aperti

- **KDF:** `ITER = 4096` giri di Ascon (era 512). Host: 0,54 ms per derivazione. Costo su GBA **da misurare**. Cambiare `ITER` rende illeggibili i file salvati.
- **Memoria:** la master password resta in RAM finché l'archivio è aperto (serve per risalvare). Alternativa: tenere solo la chiave derivata.
- **Blocchi liberi:** dopo una modifica, il file cifrato precedente resta nei blocchi liberi del file system, con la stessa chiave. Serve cancellazione sicura a livello file system.
