//! Microkernel GPU nel simulatore: VRAM propria, code di comandi fisse, nessuna condivisione
//! con la RAM della CPU. Una GPU è un dispositivo separato, non una modalità della CPU.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, sezione 8.

use crate::resource::{Arch, Error, Kind, Manifest, Microkernel, Queue};

/// Manifest del driver video generico: framebuffer e VRAM, su tutte le architetture.
pub const GPU_GENERIC: Manifest = Manifest {
    name: "gpu-generic",
    kind: Kind::Gpu,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 4 * 1024,
    queue_depth: 8,
};

/// Componente specifico per una GPU esterna: caricato solo su ARM64 e x86-64.
pub const GPU_EXTERNAL: Manifest = Manifest {
    name: "gpu-external",
    kind: Kind::Gpu,
    archs: &[Arch::Arm64, Arch::X86_64],
    required: false,
    bytes: 16 * 1024,
    queue_depth: 32,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Riempie `len` byte di VRAM da `offset` con `value`.
    Fill { offset: usize, len: usize, value: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuError {
    Full,
    OutOfVram,
}

impl From<Error> for GpuError {
    fn from(_: Error) -> Self {
        GpuError::Full
    }
}

pub struct Gpu {
    vram: Vec<u8>,
    queue: Queue<Cmd>,
}

impl Gpu {
    pub fn new(vram_bytes: usize, depth: usize) -> Self {
        Self { vram: vec![0; vram_bytes], queue: Queue::new(depth) }
    }

    /// Asincrono: il comando entra in coda, si esegue con `run`.
    pub fn submit(&mut self, cmd: Cmd) -> Result<(), GpuError> {
        self.queue.push(cmd)?;
        Ok(())
    }

    /// Esegue tutti i comandi in coda. Un comando fuori VRAM si rifiuta senza effetti.
    pub fn run(&mut self) -> Result<usize, GpuError> {
        let mut done = 0;
        while let Some(cmd) = self.queue.pop() {
            match cmd {
                Cmd::Fill { offset, len, value } => {
                    let end = offset.checked_add(len).filter(|&e| e <= self.vram.len());
                    let end = end.ok_or(GpuError::OutOfVram)?;
                    self.vram[offset..end].iter_mut().for_each(|b| *b = value);
                }
            }
            done += 1;
        }
        Ok(done)
    }

    pub fn vram(&self) -> &[u8] {
        &self.vram
    }
}

impl Microkernel for Gpu {
    fn manifest(&self) -> Manifest {
        GPU_GENERIC
    }
}
