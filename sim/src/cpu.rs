//! Microkernel CPU nel simulatore: capacità dichiarate e ammissione dei programmi.
//! Lo scheduling a budget resta in `process.rs`; qui si decide solo chi può girare.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, docs/process-management.md.

use crate::resource::{Arch, Kind, Manifest};

/// Capacità dichiarate di una CPU. Il 128 bit non esiste come voce: nessuna CPU general purpose in uso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuCaps {
    pub arch: Arch,
    pub bits: u8,
    pub cores: u8,
    pub has_mmu: bool,
}

/// Capacità note per architettura. Valori da verificare sui target reali.
pub fn caps_of(arch: Arch) -> CpuCaps {
    match arch {
        Arch::Gba => CpuCaps { arch, bits: 32, cores: 1, has_mmu: false },
        Arch::Arm64 => CpuCaps { arch, bits: 64, cores: 1, has_mmu: true }, // SMP fuori scope
        Arch::X86_64 => CpuCaps { arch, bits: 64, cores: 1, has_mmu: true },
    }
}

/// Manifest del microkernel CPU: generico, su tutte le architetture.
pub const CPU_MANIFEST: Manifest = Manifest {
    name: "cpu-generic",
    kind: Kind::Cpu,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 4 * 1024,
    queue_depth: 8,
};

/// Richiesta di un programma: quanti bit usa, quanto spazio di isolamento chiede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskReq {
    pub bits: u8,
    /// Richiede isolamento hardware (MMU). Sul GBA non è disponibile.
    pub needs_mmu: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmitError {
    WordSize,
    NoMmu,
}

/// Ammissione: un programma gira solo se la CPU ne supporta la dimensione e l'isolamento richiesto.
pub fn admit(caps: CpuCaps, task: TaskReq) -> Result<(), AdmitError> {
    if task.bits > caps.bits {
        return Err(AdmitError::WordSize);
    }
    if task.needs_mmu && !caps.has_mmu {
        return Err(AdmitError::NoMmu);
    }
    Ok(())
}
