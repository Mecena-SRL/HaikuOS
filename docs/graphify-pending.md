# Da aggiornare in Graphify

Graphify non è disponibile nell'ambiente di lavoro: questo file raccoglie ciò che va riportato quando è accessibile.

- PR #222: `docs/hardware-compatibility.md` (piano per classe hardware).
- PR #223: `x86/kernel/src/pci.rs` (enumerazione PCI bus 0).
- Ramo corrente: `x86/kernel/src/ahci.rs` (rilevamento porte AHCI, solo lettura registri), `scripts/test-qemu.sh` (disco SATA di prova in QEMU).
- Ramo corrente: `x86/kernel/src/fb.rs` (limite 4K, riempimento a span), `x86/scripts/grub.cfg` (ordine modi 4K, 1080p).
- Ramo corrente: `gui/src/genicon.rs` (icone generative, cartella L0-L2, cache), `docs/vector-generative.md`, `gui/examples/gen_icon.rs`, `design/screens/gen_folder_*.png`.
- Ramo corrente: `games/gwent/` (nuovo crate, regole Gwent, render, host).
