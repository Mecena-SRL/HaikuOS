# Motore di gioco e editor (Big Company)

> Documento di design, fase 0-1. Ciò che è **da verificare** non è ancora misurato. Il motore è `no_std` e senza dipendenze, come il resto del progetto.

## 1. Decisioni

| Tema | Decisione | Motivo |
|---|---|---|
| Linguaggio | Rust `no_std`, nessuna allocazione nel core | Stesso linguaggio dell'OS; va da GBA a ARM64/AMD64 |
| Numeri | Virgola fissa Q16.16 nella fisica | Stessi bit su ogni piattaforma: replay e test deterministici |
| Grafica v1 | Renderer software su buffer ARGB32 | Nessun driver GPU richiesto; coerente con `docs/graphics-acceleration.md` |
| Hardware GPU | Non nella prima versione | Dipende da driver e firmware; fase successiva |
| Raytracing | Fase tardiva, su CPU o GPU quando disponibile | Costo alto; prima servono luce e geometria stabili |

## 2. Fasi e capacità

| Capacità | Fase | Stato |
|---|---|---|
| Fisica AABB su griglia di tile, gravità, collisioni | 1 | Fatto (`engine/`, 4 test) |
| Disegno di mappe a tile, rettangoli, sprite con trasparenza | 1 | Fatto (`engine/`) |
| Hash dello stato per il determinismo | 1 | Fatto |
| Camera con scorrimento, sprite animati | 2 | Da fare |
| Editor di mappe e sprite | 2 | Esiste (`editor/`, PR su `games/`); da portare sul motore |
| Fisica a corpi rigidi, rotazioni | 3 | Da fare |
| Mondo 3D, mesh, rasterizzazione software | 3 | Da fare |
| Shader e GPU (Metal/Vulkan/virtio-gpu) | 4 | Da fare, dipende dai driver |
| Raytracing, luce globale | 5 | Ricerca; solo dopo la GPU |

## 3. Processo "Big Company"

Ogni fase segue: costruzione → test dei playtester → correzioni → nuovo test → rifinitura.

Stato attuale: la fase 1 è scritta da un solo agente. Il brainstorming a 45 agenti (15 ingegneri, 15 artisti, 15 playtester, un CEO Haiku) **non è ancora stato eseguito**: è costoso in token e va approvato prima.

## 4. Prossimi passi

1. Collegare `engine` al trait `Canvas` già in `gui/src/render.rs`.
2. Portare la scena dell'editor (`HGS2`) sul motore.
3. Definire il formato dei giochi e il primo gioco di prova (platformer, già esistente in `games/platformer`).

Test: `cargo test -p haiku-engine`.
