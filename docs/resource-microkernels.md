# Microkernel per risorsa

> Stato: bozza di ricerca e design. Nessun codice per hardware reale. Il prototipo è in `sim/src/resource.rs` (host, 8 test). Ogni ipotesi non verificata è segnata **da verificare**.
> Legato a [architecture.md](architecture.md) (sezioni 2.2, 2.3, 9) e a [portability.md](portability.md) (regole di rilevamento e installazione atomica).

## 1. Sintesi

- **Un microkernel per risorsa:** CPU, GPU/video, memoria RAM/VRAM, storage, swap e compressione, audio, input/periferiche, alimentazione, crittografia.
- **Un contratto comune:** ogni microkernel dichiara cosa fa, dove gira e quanto costa, e parla con gli altri solo con richieste asincrone a coda fissa.
- **Più driver insieme:** si caricano più microkernel per la stessa risorsa (es. GPU interna più NVIDIA e AMD). Ognuno ha coda e stato propri; avanzano in parallelo senza condividere memoria.
- **Generico e specifico:** ogni risorsa ha un driver generico che funziona ovunque con il minimo. I micro-componenti specifici per hardware si caricano sopra, solo se il rilevamento li trova.
- **Architetture:** 32 bit (ARM7TDMI sul GBA), 64 bit (ARM64, Apple Silicon compreso, e x86-64). **Non esistono CPU general purpose a 128 bit in uso**: il 128 bit non è un target. Le GPU non sono una "modalità" della CPU: sono un altro dispositivo con il proprio microkernel, con memoria e linguaggio di comando propri.

## 2. Fonti e stato dell'arte

| Sistema | Cosa prendiamo | Cosa scartiamo | Fonte |
|---|---|---|---|
| seL4 | Driver in spazio utente, interrupt e DMA come capability concesse dal kernel; framework sDDF con code condivise e notifiche | Verifica formale e modello di capability completo (troppo per GBA) | [sDDF design](https://trustworthy.systems/projects/drivers/sddf-design.pdf), [sDDF su GitHub](https://github.com/au-ts/sddf), [seL4 manual](https://www.sel4.systems/Info/Docs/seL4-manual-latest.pdf) |
| Zircon / Fuchsia | Driver come componenti con dipendenze dichiarate (DFv2); accesso tramite handle | Gli oggetti kernel generici e il runtime di componenti | [Driver framework (DFv2)](https://fuchsia.dev/docs/concepts/drivers/driver_framework) |
| Linux DRM | Scheduler di GPU a code per contesto, dipendenze tra lavori, memoria gestita dal driver | Il modello monolitico e la complessità del device model | [DRM GPU scheduler](https://sources.debian.org/src/linux/6.17.7-2/drivers/gpu/drm/scheduler/sched_main.c), [drm-internals](https://docs.kernel.org/gpu/drm-internals.html) |
| Redox | Driver in user space come servizi (scheme) | Il design UNIX completo | [Redox graphics e driver](https://doc.redox-os.org/book/ch04-09-graphics-windowing.html) |
| Vulkan | Modello di accesso GPU: command buffer, code di lavoro, sincronizzazione esplicita con fence e semafori | Il ruolo di API grafica completa | [Vulkan spec](https://docs.vulkan.org/spec/latest/) |
| Metal (Apple) | Comandi raggruppati per coda, memoria condivisa unificata su Apple Silicon | Dipendenza dall'ecosistema Apple | [Metal](https://developer.apple.com/metal/) (da verificare il dettaglio sulle code) |

Nota: le fonti sono state trovate con la ricerca web. Le pagine principali sono state verificate solo per titolo e contenuto generale; i dettagli di API sono **da verificare** sui documenti ufficiali prima di fissare una specifica.

## 3. Contratto comune

Ogni microkernel per risorsa espone:

1. **Manifest (dichiarato, statico):** `name`, `kind` (CPU, GPU, memoria, storage, swap, audio, input, alimentazione, crittografia), `archs` (architetture supportate), `required` (indispensabile o no), `bytes` (RAM necessaria), `queue_depth` (dimensione della coda).
2. **Capacità (dichiarate):** cosa sa fare e i limiti (es. "GPU: 2 code, max 64 MB VRAM"). Il monolite le legge, non le modifica.
3. **Stato:** `Ready`, `Running`, `Stopped`. Solo il monolite cambia lo stato di un driver.
4. **Richieste asincrone:** `submit(richiesta) -> Ok | Full | Stopped`. La richiesta entra in coda e il completamento arriva dopo, con `id`. Nessuna chiamata bloccante.
5. **Errori:** un insieme chiuso (`Full`, `Stopped`, più quelli specifici di risorsa). Un errore non ferma gli altri driver.

Regole del contratto:
- Coda a dimensione fissa, nessuna allocazione dinamica dopo l'avvio (come `docs/process-management.md`).
- Il completamento ritorna allo stesso driver che ha fatto la richiesta. Nessun driver legge le code degli altri.
- Un driver non conosce gli altri driver della stessa risorsa: il coordinamento passa dal monolite.

## 4. Microkernel per risorsa

| Risorsa | Generico (sempre) | Specifico (se rilevato) | Architetture generiche | Note |
|---|---|---|---|---|
| **CPU** | pianificazione a priorità e budget (vedi process-management) | tabelle di scheduling per core (futuro SMP) | GBA, ARM64, x86-64 | Su GBA un solo core. Il timer è il livello basso. |
| **GPU / video** | framebuffer lineare o modo video semplice | driver per GPU NVIDIA, AMD, integrata | GBA: VRAM (modi 3/4). ARM64/x86: framebuffer UEFI | Una GPU è un dispositivo a sé con la sua memoria. Vedi graphics.md. |
| **Memoria RAM / VRAM** | pool a blocchi fissi, quote dal monolite | gestione VRAM del driver GPU | tutte | La VRAM è una quota separata, con revoca. |
| **Storage (SSD, dischi)** | blocchi, checksum metadati (vedi filesystem-roadmap) | driver NVMe, SATA, SD, flash GBA | GBA: SRAM/flash cartuccia. ARM64/x86: blocchi | Ogni disco è un driver: più dischi = più istanze. |
| **Swap e compressione** | compressione offline (LZSS) su GBA | compressione runtime (LZ4/Zstd) su macchine potenti | tutte, con limiti diversi | Vedi memory-security.md. Swap solo dove c'è un disco. |
| **Audio** | mixer a campioni fissi | driver specifici per codec | GBA: registri audio. ARM64/x86: buffer | Un driver di mix per programma è un'ipotesi **da verificare**. |
| **Input / periferiche** | tastiera e pulsanti (GBA), polling | USB, mouse, touch, controller | tutte | Ogni periferica dichiara i suoi eventi. |
| **Alimentazione** | stato batteria e modi di riposo | regolatori specifici | GBA: nessuno. ARM64: da verificare. x86: da verificare | Non tocca la CPU direttamente: chiede al monolite. |
| **Crittografia** | Ascon-AEAD128 (ovunque) | AES hardware, ChaCha20 software | tutte | Vedi memory-security.md. |

## 5. Regole di caricamento

1. **Rilevamento:** all'avvio il livello basso costruisce `Target { arch, ram_free }`. Architettura sconosciuta: nulla caricato (fail closed).
2. **Verifica (`plan`):** per ogni manifest, se l'architettura è supportata lo si attiva; se è incompatibile e `required`, blocca l'installazione; se è opzionale, lo si salta e lo si registra nel log. RAM insufficiente blocca tutto. Nessuna modifica di stato.
3. **Attivazione (`activate`):** in ordine di dipendenza. Se una attivazione fallisce, si fermano quelle già attive in ordine inverso e lo stato resta quello precedente.
4. **Specifici sopra il generico:** un micro-componente per hardware (es. "GPU NVIDIA X") si carica solo se il rilevamento lo trova, e dichiara le sue capacità e i suoi limiti. Se manca, resta il driver generico.
5. **Più driver per la stessa risorsa:** ammessi. Ogni istanza ha id proprio e coda propria. Il monolite assegna le richieste; non c'è un driver "principale" fisso.

## 6. Regole di isolamento

- **GBA:** isolamento solo logico (decisione di Ivan, `docs/process-management.md`). I driver sono nello stesso spazio; il monolite e Rust fanno rispettare le code e i confini.
- **ARM64 e x86-64:** ogni driver in un proprio microkernel con MMU. Accesso all'hardware (registri, DMA, interrupt) solo tramite capability concesse dal monolite (ispirato a seL4). **Da verificare** come gestire il DMA su ogni piattaforma (IOMMU).
- Un crash di un driver non tocca gli altri: il monolite lo ferma (`Stopped`) e revoca le sue quote.
- Memoria condivisa tra driver: solo con una regola esplicita dell'allowlist del monolite.

## 7. Cosa è generico e cosa è specifico

- **Generico (codice comune, Rust `no_std`):** il contratto, la coda, il manifest, `plan`, `activate`, la gestione degli errori.
- **Specifico (livello basso):** registri, interrupt, DMA, firmware, formato dei comandi della GPU. Non entra nel codice comune.
- **Specifico per hardware (micro-componenti):** dichiarano capacità e limiti, non cambiano il contratto.

## 8. GPU non è una modalità della CPU

Una GPU ha memoria propria (VRAM o memoria unificata con accesso dichiarato), code di lavoro, sincronizzazione asincrona e linguaggio di comando specifico. Per questo ha un microkernel suo. La CPU la comanda con richieste, non la esegue. Su Apple Silicon la memoria è unificata: la differenza è una capacità dichiarata (**da verificare** il dettaglio), non una eccezione al contratto.

## 9. Prototipo (`sim/`)

- `sim/src/resource.rs`: `Arch`, `Kind`, `Manifest`, `Driver` (coda fissa, `submit`/`step`/`stop`), `plan`, `activate` con rollback, `tick`.
- `sim/tests/risorse.rs`: 8 test, tutti passati. Coprono: opzionale incompatibile saltato, richiesto incompatibile bloccante, RAM insufficiente, architettura sconosciuta, rollback in ordine inverso, due GPU in parallelo senza mescolare i risultati, coda piena, stop di un driver senza effetti sugli altri.
- Il lavoro simulato è `op + 1`. Nessuna misura di prestazioni.

## 10. Da verificare

- Dimensione di coda e numero di driver per risorsa sulla GBA (RAM 288 KB).
- Modello di DMA e IOMMU su ARM64 e x86-64.
- Gestione della memoria unificata di Apple Silicon e della VRAM separata.
- Forma delle capability dichiarate (formato binario o testo) e loro versione.
- Driver audio: quanti buffer servono per non avere scatti.
- Alimentazione su ARM64 e x86-64: quali stati esistono davvero.
- Ordine di attivazione tra risorse (es. memoria prima di GPU): oggi è fisso nel manifest.
