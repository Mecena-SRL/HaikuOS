# Accelerazione grafica per HaikuOS

> Documento di decisione. Ciò che è segnato **"da verificare"** è un'ipotesi non ancora misurata. Nessun codice in questa fase.

## 1. Raccomandazione

Partire dal basso, in quest'ordine:

1. **Framebuffer lineare via UEFI GOP** su x86-64 e ARM64: è lo schermo minimo, funziona ovunque ci sia UEFI.
2. **Rendering software con SIMD** (SSE2 su x86-64, NEON su ARM64): blit, riempimenti e rasterizzazione vettoriale su CPU. È la vera "accelerazione" della prima versione.
3. **virtio-gpu** per le macchine virtuali (QEMU), utile per test automatici.
4. **Driver GPU nativo** solo dopo, e solo per una famiglia (Intel). AMD e NVIDIA non sono un obiettivo iniziale.

Motivo: il desktop generico (primo uso, vedi `docs/architecture.md` §8) non richiede 3D. Un percorso CPU+SIMD è piccolo, testabile e non richiede firmware proprietario. Coerente con priorità su sicurezza e semplicità.

## 2. Opzioni

| Opzione | Dove | Costo | Pro | Contro |
|---|---|---|---|---|
| GOP framebuffer | x86-64, ARM64 (UEFI) | Basso | Standard, nessun driver | Solo scrittura pixel, nessuna accelerazione |
| Blit/fill SIMD su CPU | Ovunque | Medio | Portabile, sicuro, testabile | Limitato da banda di memoria |
| virtio-gpu (2D) | VM / QEMU | Basso–medio | Spec aperta, comandi semplici | Serve solo in virtuale |
| Intel iGPU (i915/Xe) | PC Intel | Alto | Accesso reale all'hardware | Documentazione parziale, molto codice |
| AMD (amdgpu) | PC AMD | Molto alto | — | Firmware complesso, nessun obiettivo iniziale |
| NVIDIA | PC NVIDIA | Molto alto | — | Driver proprietario, firmware chiuso |
| Game Boy Advance | GBA | Nessuno | Hardware dedicato a tile e sprite | Nessun framebuffer GPU: usa la modalità video del chip |

## 3. Note per arco

- **x86-64 e ARM64:** GOP è disponibile in entrambi tramite firmware UEFI. Il framebuffer è di solito memoria "write-combining": leggere da lì è lento, quindi il rendering va fatto in un buffer RAM e copiato una volta per frame. (da verificare: prestazioni reali su hardware)
- **Game Boy Advance:** non ha GPU né framebuffer lineare di uso generico. Il percorso è quello del chip (tile, sprite, modalità 3). Non è un problema di questo documento.

## 4. Impatto sull'architettura

- La grafica è un **microkernel hardware verticale** (`docs/architecture.md` §2.2). Il monolite decide solo quote e permessi, non come si disegna.
- Il framebuffer viene assegnato come **quota** a un solo verticale. I programmi non lo toccano direttamente: passano dal verticale grafico con messaggi a regole fisse.
- Il percorso software resta sempre disponibile: se manca un driver GPU, il sistema funziona comunque.

## 5. Piano a fasi

| Fase | Obiettivo | Uscita verificabile |
|---|---|---|
| 0 | Schermo via GOP, pixel e testo | Desktop statico in QEMU x86-64 e ARM64 |
| 1 | Blit, fill, rettangoli e rasterizzazione vettoriale con SIMD | Benchmark di riempimento a 640×480 e 1920×1080 (da verificare) |
| 2 | Driver virtio-gpu 2D | Desktop in QEMU con flush su host |
| 3 | Driver Intel iGPU, solo 2D (blitter) | Stesso desktop su un PC Intel reale |
| 4 | Eventuale 3D o altre famiglie | Da decidere dopo la fase 3 |

Nota: le fasi 0–2 non richiedono documentazione proprietaria. La fase 3 dipende dalla disponibilità di documentazione Intel.

## 6. Riferimenti

- [OSDev: GOP](https://wiki.osdev.org/GOP)
- [OSDev: Native Intel graphics](https://wiki.osdev.org/Native_Intel_graphics)
- [virtio-gpu: specifica OASIS (lista virtio-dev)](https://lists.oasis-open.org/archives/virtio-dev/202206/msg00056.html)
- [Linux GPU Driver Developer's Guide](https://docs.kernel.org/7.1/gpu/index.html)
