# Ambiente di sviluppo di HaikuOS

> Stato: prima versione. Solo host (std). Nessun IDE grafico ancora: questo documento fissa le scelte e il primo nucleo verificato.

## 1. Pezzi

| Pezzo | Scopo | Stato |
|---|---|---|
| Manifest `haiku.toml` | Descrive un pacchetto: nome, versione, architetture, dipendenze | fatto (`devtools/src/manifest.rs`) |
| Ordine di build | Compila dipendenze prima dei pacchetti, rileva cicli e dipendenze mancanti | fatto (`devtools/src/graph.rs`) |
| Verifica pacchetti | SHA-256 del payload prima dell'installazione | fatto (`devtools/src/pkg.rs`) |
| Package manager | Indice, risoluzione dipendenze, installazione atomica | fatto (`devtools/src/pm.rs`) |
| Build runner | Piano per architettura ed esecuzione dei comandi, in ordine | fatto (`devtools/src/build.rs`, CLI `hdev build <arch> <dir>...`) |
| IDE | Editor, albero progetto, errori di build, avvio in emulatore | da fare (dopo il GUI desktop) |

Il codice dell'OS resta `no_std`. `devtools/` gira sull'host e non entra nel sistema.

## 2. Formato pacchetto

```toml
[package]
name = "calc"
version = "0.1.0"
archs = ["gba", "x86_64"]   # obbligatorio: regola di portabilità
deps = ["ui"]
build = "make calc"          # opzionale: comando eseguito con sh -c
```

Regole:
1. `archs` vuoto è un errore.
2. Ogni dipendenza deve esistere nell'indice.
3. Nessun ciclo.
4. Il pacchetto installato deve avere il digest SHA-256 dichiarato nell'indice. Altrimenti non si installa.

## 3. Installazione atomica

Come da `docs/portability.md` (regola 3): si installa tutto o nulla.
Il pacchetto viene verificato, poi copiato in un'area temporanea, poi spostato nel posto definitivo in un solo passo.

## 4. Indice

```toml
[[pkg]]
name = "calc"
version = "0.1.0"
sha256 = "..."
deps = ["ui"]
```

## 5. Domande aperte

1. Formato dell'archivio: tar semplice o formato proprio? (proposta: formato proprio, più piccolo)
2. Firma dei pacchetti: chiave del monolite o chiave del produttore? (proposta: chiave del produttore, verificata dal monolite)
3. Quando entra il build runner nell'IDE: prima della GUI o dopo?

## 6. Come provare

```sh
cd devtools && cargo test
```
