# Gestione dei processi di HaikuOS

Stato: proposta di ricerca. Nessun codice per hardware reale. Il prototipo vive in `sim/` (host).

## 1. Sintesi

- **Tabella fissa:** i processi stanno in una tabella di dimensione fissa, uguale al limite del monolite (`MAX_PROGRAMS`). Nessuna allocazione dinamica dopo l'avvio.
- **Scheduling:** priorità fisse, round-robin a parità di priorità, budget di CPU per turno. È il modello più semplice che limita davvero il tempo di ogni programma.
- **Comunicazione:** solo tramite il monolite, con allowlist e messaggi a dimensione fissa (già in `sim/`).
- **Ammissione:** un processo nasce solo se il monolite lo registra e gli concede una quota.
- **Isolamento:** dipende dalla piattaforma. Su GBA non c'è hardware che lo garantisca (vedi sezione 5).

## 2. Stato dell'arte (fonti)

| Sistema | Idea utile per noi | Cosa scartiamo | Fonte |
|---|---|---|---|
| seL4 (MCS) | Il tempo di CPU è una capability (scheduling context) con budget e periodo: il limite è verificabile | Il costo di gestire capability di tempo su GBA | [Scheduling context capabilities](https://sel4.systems/Research/pdfs/scheduling-context-capabilities.pdf), [MCS tutorial](https://docs.sel4.systems/Tutorials/mcs) |
| Hubris (Oxide) | Insieme di task fissato a compile time, nessuna creazione dinamica, IPC sincrona | Il modello a task statici senza ammissione a runtime (noi ammettiamo a runtime con quote) | [Hubris reference](https://hubris.oxide.computer/reference/) |
| Tock | Processi caricati da flash, isolamento con MPU, kernel in Rust | Il kernel che esegue codice non fidato nello stesso spazio (noi non lo facciamo) | [Tock, levy17](https://www.eecs.berkeley.edu/~prabal/pubs/papers/levy17tock.pdf), [TickTock 2025](https://cseweb.ucsd.edu/~dstefan/pubs/rindisbacher:2025:ticktock.pdf) |
| Zircon (Fuchsia) | Job come contenitori di processi e risorse, accesso tramite handle (capability) | Gli oggetti kernel generici e troppo grandi per 288 KB | [Zircon concepts](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/docs/concepts/kernel/concepts.md) |
| Redox OS | Driver e file system in spazio utente tramite "scheme" | Il design UNIX completo (troppo per GBA) | [FOSDEM 2025, Redox](https://fosdem.org/2025/events/attachments/fosdem-2025-5973-redox-os-a-microkernel-based-unix-like-os/slides/238806/redoxos-a_VThTapJ.pdf) |
| QNX | Partizioni con budget garantito (adaptive partitioning) | Il dettaglio commerciale e la dimensione del sistema | Da verificare sulla documentazione ufficiale QNX |

Nota: le fonti sopra sono state trovate con la ricerca; i dettagli di implementazione oltre quanto riportato nelle pagine sono da verificare prima di usarli come specifica.

## 3. Modello proposto

### 3.1 Processi
- Tabella di `MAX_PROCS` voci (4 su GBA). Stati: `Ready`, `Running`, `Blocked`. L'uscita libera lo slot.
- Creazione: `spawn` rifiuta duplicati, tabella piena e budget zero.

### 3.2 Scheduling
- Vince il pronto a priorità più alta (valore più basso). A parità, round-robin: vince chi ha aspettato di più.
- **Budget per turno:** un processo mantiene la CPU per al massimo `budget` tick, poi torna in coda. Nessun programma può occupare la CPU per sempre.
- **Preemption:** un risveglio o una creazione con priorità più alta prende la CPU subito. A parità non ruba la CPU.
- **Idle:** se nessuno è pronto, la CPU è libera.
- **Tick:** su GBA la base naturale è l'interrupt VBlank (circa 60 Hz, cioè circa 16,7 ms). **Da verificare** con i tempi reali.

### 3.3 Comunicazione
- Solo tramite il monolite: allowlist fissa, messaggi di 64 byte, mittente e destinatario devono avere uno slot.
- Un messaggio non concede né modifica diritti.

### 3.4 Ammissione
- `spawn` del kernel di processi va chiamato solo dopo `register` e la quota del monolite. L'agente propone, il monolite decide.

## 4. Alternative scartate

- **Scheduling a quanto fisso senza priorità:** semplice, ma un programma lento o bloccato rallenta il sistema. Scartato.
- **Scheduling con budget periodici completi (stile seL4 MCS) su GBA:** più preciso, ma richiede timer e contabilità più grandi del budget della RAM. Lo teniamo come obiettivo per la macchina potente.
- **Task statici come Hubris:** non compatibile con gli agenti che chiedono quote a runtime. Scartato come modello generale.
- **Thread e multicore ora:** GBA ha un solo core. Rimandato (vedi `docs/architecture.md`, sezione 11).

## 5. Limiti per piattaforma

- **GBA (ARM7TDMI):** nessuna MMU, nessuna MPU. L'isolamento tra programmi è solo logico: il monolite e il linguaggio (Rust) lo fanno rispettare, ma un programma difettoso può scrivere in memoria altrui. Va deciso se accettarlo (vedi sezione 8).
- **ARM64 e x86-64:** MMU disponibile. Ogni programma ha uno spazio di indirizzi proprio; il modello sopra resta uguale.

## 6. Prototipo e test (`sim/`)

- `sim/src/process.rs`: tabella fissa, scheduling a priorità e budget, block/wake, exit.
- `sim/tests/processi.rs`: 11 test. Coprono tabella piena, duplicati, budget zero, round-robin, preemption, risveglio senza furto della CPU, idle, uscita che libera lo slot.
- Eseguiti con `cargo test` in `sim/`: tutti passano (8 test esistenti più 11 nuovi).
- Non verificato: prestazioni, comportamento su hardware, interrupt reali.

## 7. Da verificare

- Durata del tick VBlank e del budget in tick su GBA (misure in mGBA).
- Dimensione della tabella processi e dei metadati in IWRAM o EWRAM.
- Dettagli di QNX e di Tock citati sopra.
- Collegamento tra `Scheduler` e `Monolith` (oggi sono due moduli separati nel simulatore).

## 8. Decisioni per Ivan

1. **Isolamento su GBA:** accettare l'isolamento solo logico su GBA (più semplice, più piccolo) oppure limitare il sistema alle piattaforme con MMU/MPU?
