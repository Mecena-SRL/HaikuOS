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
| `office/src/sheet.rs` | host, no_std | formats, zip | XLSX (primo foglio) e ODS in sola lettura, test inline |
| `devtools/` | host, std | - | manifest, ordine di build, build runner, package manager, nucleo IDE, progetto, SHA-256, CLI `hdev` |

## Stato

- `devtools/`: manifest, ordine di build, verifica SHA-256 (#158 unita).
- Build runner: `build.rs` + `hdev` (#162 unita).
- Package manager: `pm.rs` (indice, risoluzione, installazione atomica; questa PR).
- IDE: nucleo `ide.rs` (buffer con annulla) e `project.rs` (file e load/save; questa PR). GUI da fare: la finestra la fornisce il desktop.
