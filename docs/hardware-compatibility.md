# Compatibilità hardware

> Stato: piano di lavoro. Nessun driver nuovo in questa PR. Tutto ciò che non è stato letto sulla fonte ufficiale è segnato **da verificare**. Nel sandbox non ci sono hardware reali né accesso alle specifiche (proxy): ogni voce "verificabile" qui vuol dire solo "testabile nel simulatore".
> Legato a [resource-microkernels.md](resource-microkernels.md) (contratto e regole di caricamento), a [compatibility.md](compatibility.md) (file system e partizioni) e a [architecture.md](architecture.md).

## 1. Principio

- Ogni classe di hardware è una **risorsa** con il suo microkernel (vedi resource-microkernels.md §4.1).
- Ogni risorsa ha **un driver generico** che usa solo standard (la base che fa girare quasi tutto) e **driver specifici** caricati solo se il rilevamento li trova.
- Il rilevamento usa gli standard di bus e firmware (ACPI, PCI/PCIe, USB, device tree su ARM): niente liste di modelli nel nucleo.
- Se manca un driver specifico, il sistema resta utilizzabile con il generico, oppure dichiara la periferica "non supportata" nel log. Mai un blocco silenzioso.

## 2. Matrice per classe

Priorità: **P1** avvio e uso base; **P2** sistema utilizzabile; **P3** più avanti.

| Classe | Rilevamento | Driver generico (standard) | Driver specifici | Priorità | Verificabile nel sim |
|---|---|---|---|---|---|
| **CPU Intel/AMD (x86-64)** | CPUID, ACPI (MADT) | Avvio UEFI, long mode, timer e interrupt APIC | Microcodice, tabelle per famiglia (solo dove serve) | P1 | No (serve hardware o emulatore) |
| **CPU ARM64** | Device tree / ACPI, registri ID | Avvio UEFI, EL1, GIC, timer generico | Per SoC: solo tabelle, nessun codice | P1 | Parziale (QEMU `virt`) |
| **CPU ARM7 (GBA)** | Fisso (target noto) | Modo 32 bit, timer, IRQ del GBA | Nessuno | P1 | Sì (già in `gba/`) |
| **Microcontrollori (MCU)** | Target di compilazione | Avvio, timer, UART, GPIO | Per famiglia: ARM Cortex-M, RISC-V (da verificare) | P3 | No |
| **GPU** | PCI (classe 0x03), ACPI | Framebuffer UEFI (GOP) o modo lineare; nessuna accelerazione | NVIDIA, AMD, Intel integrata: DRM-like, a comandi asincroni | P1 generico, P3 specifici | Solo framebuffer |
| **Accelerazione grafica** | Come sopra | Rendering software (sempre) | Vulkan/compute tramite driver specifico | P2 | No |
| **Risoluzione e più schermi** | EDID via DDC, GOP | Scala a risoluzione nativa; fallback 640×480 | Modi multipli, HDMI/DP specifici | P2 | Solo fallback |
| **Audio** | PCI (HDA, 0x0403), USB Audio Class | Intel HDA (quasi universale) + USB Audio Class | Codec specifici (Realtek, ecc.) | P2 | No |
| **Wi-Fi / Bluetooth** | PCI, USB | Solo rete cablata (ethernet) come base | Chip per chip; firmware redistribuibile **da verificare** | P2 | Frame ethernet: sì (`sim/src/net.rs`) |
| **Ethernet** | PCI, USB | Driver per controller standard (e1000-like, virtio-net) | Per scheda | P1 | Frame: sì |
| **SSD / dischi** | PCI classe 0x01: NVMe (0x0108), AHCI (0x0106), USB Mass Storage | NVMe e AHCI (standard, coprono quasi tutto) | Solo se una scheda non segue lo standard | P1 | Blocchi: sì (`storage.rs`) |
| **RAM** | UEFI memory map, ACPI | Mappa memoria dal firmware, pool a blocchi | Nessuno | P1 | Sì (`memory.rs`) |
| **Virtualizzazione** | CPUID (VMX/SVM), ID ARM (EL2) | Nessuna necessità per avviare; rilevamento e stato | KVM-like: solo dopo P2 | P3 | No |
| **Tastiera** | USB HID, PS/2 (i8042), UEFI Simple Text Input | HID boot protocol (standard) | Layout per lingua (tabelle dati, non codice) | P1 | Eventi: sì (`input.rs`) |
| **Mouse / touchpad** | USB HID, PS/2 | HID boot protocol | Precisione touchpad (multi-touch HID) | P1 mouse, P2 touchpad | Eventi: sì |
| **Controller / GBA pulsanti** | Fisso | Registri tasti | Gamepad USB HID | P1 (GBA), P2 (USB) | Sì |
| **USB (host)** | PCI: xHCI (0x0C03 30) | xHCI, EHCI/OHCI solo se servono | Hub e dispositivi via HID/classi standard | P1 | No |
| **Stampanti** | USB Printer Class, rete (IPP Everywhere) | IPP Everywhere via rete (standard) | Driver PCL/PostScript solo se IPP non basta | P2 | No |
| **Schermi touch** | USB HID, I2C-HID | HID standard | Per controller | P3 | No |

## 3. Ordine di lavoro

1. **P1 generico**: CPU (x86-64, ARM64), RAM dal firmware, storage (NVMe, AHCI), tastiera e mouse HID, framebuffer GOP, xHCI. È il minimo per avviare un programma.
2. **P2**: audio HDA e USB Audio, ethernet, tastiera PS/2, stampanti via IPP, accelerazione software, risoluzioni multiple.
3. **P3**: GPU specifiche, Wi-Fi/Bluetooth per chip, MCU, virtualizzazione, touch I2C.

Ogni passo parte dalla **lettura** della specifica ufficiale (UEFI, ACPI, PCI, NVMe, AHCI, USB HID, HDA, IPP) prima del codice. Finché la fonte non è letta, la voce resta **da verificare**.

## 4. Cosa si può verificare senza hardware

- Frame, blocchi, coda input, memoria: sì, nel simulatore (`sim/`) e nei test già presenti.
- Rilevamento PCI/USB: sì, su tabelle di dati scritte nel test (es. ID di classe), non su hardware.
- Avvio e framebuffer: solo in QEMU (x86-64 e ARM64 `virt`).
- Driver reali (GPU, Wi-Fi, audio, stampanti): **no**. Serve una macchina reale o una VM con dispositivo passato.

## 5. Regole

1. Niente liste di modelli nel nucleo: solo classi e ID di bus.
2. Ogni driver specifico dichiara `archs` e `bytes` nel manifest (contratto in resource-microkernels.md §3).
3. Un driver mancante non blocca l'avvio: se è `required` il rilevamento si ferma con un messaggio chiaro; altrimenti si registra e si prosegue.
4. Firmware proprietari: caricati solo se redistribuibili; altrimenti la scheda resta col generico. **Da verificare** per ogni chip.

## 6. Da verificare

- Testi ufficiali di UEFI, ACPI, PCI, NVMe, AHCI, USB HID, HDA, IPP: non letti (sandbox senza accesso).
- Quanta RAM serve per il driver GPU generico su ARM64 e x86-64.
- Licenze dei firmware Wi-Fi e GPU.
- Se IOMMU è sempre presente per il DMA; senza, il driver storage resta in user space solo logico.
