# Grafo del progetto (Graphify)

Mappa viva di moduli e dipendenze. Va aggiornata ad ogni PR.

## Moduli

| Modulo | Tipo | Dipende da | Ruolo |
|---|---|---|---|
| `kernel/` | OS, x86 32 bit | - | kernel Multiboot2, primo avvio su COM1 |
| `setup/` | OS, `no_std` | - | logica primo avvio |
| `sysapps/` | OS, `no_std` | - | browser ed editor di script |
| `sim/` | host | - | simulatore monolite, agente, filtro messaggi |
| `gba/` | OS, Game Boy Advance | - | ROM GBA |
| `gui/` | host | - | desktop e window manager |
| `devtools/` | host, std | - | manifest, ordine di build, build runner, SHA-256, CLI `hdev` |

## Stato

- `devtools/`: manifest, ordine di build, verifica SHA-256 (#158 unita).
- Build runner: `build.rs` + `hdev` (questa PR).
- Package manager: da fare.
