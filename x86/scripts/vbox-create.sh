#!/usr/bin/env bash
# Crea una VM VirtualBox "HaikuOS" che avvia build/haikuos.iso (BIOS, 256 MB, VGA 1280x800).
# Uso: ./scripts/vbox-create.sh [percorso_iso]   (poi: VBoxManage startvm HaikuOS)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ISO="${1:-$ROOT/build/haikuos.iso}"
VM=HaikuOS
VBoxManage createvm --name "$VM" --ostype Other_64 --register
VBoxManage modifyvm "$VM" --memory 256 --firmware bios --boot1 dvd --boot2 none --boot3 none --boot4 none \
  --graphicscontroller vmsvga --vram 16 --audio-driver none --nic1 none
VBoxManage storagectl "$VM" --name IDE --add ide
VBoxManage storageattach "$VM" --storagectl IDE --port 0 --device 0 --type dvddrive --medium "$ISO"
echo "VM $VM creata con $ISO. Avvio: VBoxManage startvm $VM"
