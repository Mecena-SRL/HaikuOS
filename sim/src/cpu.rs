//! Microkernel CPU nel simulatore: capacità dichiarate e ammissione dei programmi.
//! Lo scheduling a budget resta in `process.rs`; qui si decide solo chi può girare.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, docs/process-management.md.

use crate::resource::{Arch, Kind, Manifest, Microkernel, Status};

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
    /// Il microkernel è fermo.
    Halted,
    /// Tabella dei programmi piena.
    Full,
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

/// Microkernel CPU con stato: conta i programmi ammessi, può essere fermato.
pub struct Cpu {
    caps: CpuCaps,
    max_tasks: usize,
    admitted: usize,
    running: bool,
}

impl Cpu {
    pub fn new(arch: Arch, max_tasks: usize) -> Self {
        Self { caps: caps_of(arch), max_tasks, admitted: 0, running: true }
    }

    pub fn caps(&self) -> CpuCaps {
        self.caps
    }

    /// Ammette un programma se c'è posto e la CPU lo supporta. Ritorna il numero di programmi ammessi.
    pub fn admit_task(&mut self, task: TaskReq) -> Result<usize, AdmitError> {
        if !self.running {
            return Err(AdmitError::Halted);
        }
        admit(self.caps, task)?;
        if self.admitted >= self.max_tasks {
            return Err(AdmitError::Full);
        }
        self.admitted += 1;
        Ok(self.admitted)
    }

    pub fn release_task(&mut self) {
        self.admitted = self.admitted.saturating_sub(1);
    }

    /// Ferma la CPU: nessun nuovo programma. I programmi già ammessi restano.
    pub fn halt(&mut self) {
        self.running = false;
    }
}

impl Microkernel for Cpu {
    fn manifest(&self) -> Manifest {
        CPU_MANIFEST
    }
    fn status(&self) -> Status {
        if self.running { Status::Ready } else { Status::Stopped }
    }
}
