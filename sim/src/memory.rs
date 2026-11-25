//! Microkernel memoria nel simulatore: pool RAM e VRAM separati, quote per proprietario,
//! revoca che azzera prima del riuso. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md, docs/process-management.md (sezione 3.5).

use crate::resource::{Kind, Manifest, Arch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool {
    Ram,
    Vram,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pool: Pool,
    slot: usize,
    owner: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemError {
    OverQuota,
    NoSpace,
    BadHandle,
    /// Il proprietario non possiede il blocco.
    NotOwner,
}

/// Manifest del microkernel memoria. VRAM è un pool separato, non un'altra RAM.
pub const MEMORY_MANIFEST: Manifest = Manifest {
    name: "memory-generic",
    kind: Kind::Memory,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 2 * 1024,
    queue_depth: 4,
};

struct Block {
    data: Vec<u8>,
}

pub struct MemoryKernel {
    ram: Vec<Option<Block>>,
    vram: Vec<Option<Block>>,
    block_bytes: usize,
    /// Quota in blocchi per proprietario; indice = id proprietario.
    quota: Vec<usize>,
    used: Vec<usize>,
}

impl MemoryKernel {
    /// `ram_blocks` e `vram_blocks`: numero di blocchi fissi per pool. Nessuna allocazione dopo `new`.
    pub fn new(ram_blocks: usize, vram_blocks: usize, block_bytes: usize, owners: usize) -> Self {
        Self {
            ram: (0..ram_blocks).map(|_| None).collect(),
            vram: (0..vram_blocks).map(|_| None).collect(),
            block_bytes,
            quota: vec![0; owners],
            used: vec![0; owners],
        }
    }

    /// Il monolite concede una quota (in blocchi) a un proprietario. Revoca con 0.
    pub fn set_quota(&mut self, owner: u8, blocks: usize) {
        self.quota[owner as usize] = blocks;
    }

    fn pool_mut(&mut self, pool: Pool) -> &mut Vec<Option<Block>> {
        match pool {
            Pool::Ram => &mut self.ram,
            Pool::Vram => &mut self.vram,
        }
    }

    pub fn alloc(&mut self, owner: u8, pool: Pool) -> Result<Handle, MemError> {
        let o = owner as usize;
        if self.used[o] >= self.quota[o] {
            return Err(MemError::OverQuota);
        }
        let block_bytes = self.block_bytes;
        let slots = self.pool_mut(pool);
        let slot = slots.iter().position(|b| b.is_none()).ok_or(MemError::NoSpace)?;
        slots[slot] = Some(Block { data: vec![0; block_bytes] });
        self.used[o] += 1;
        Ok(Handle { pool, slot, owner })
    }

    fn block_mut(&mut self, h: Handle, caller: u8) -> Result<&mut Block, MemError> {
        if h.owner != caller {
            return Err(MemError::NotOwner);
        }
        let b = self.pool_mut(h.pool).get_mut(h.slot).and_then(|b| b.as_mut());
        b.ok_or(MemError::BadHandle)
    }

    pub fn write(&mut self, h: Handle, caller: u8, at: usize, bytes: &[u8]) -> Result<(), MemError> {
        let b = self.block_mut(h, caller)?;
        let end = at.checked_add(bytes.len()).filter(|&e| e <= b.data.len());
        let end = end.ok_or(MemError::BadHandle)?;
        b.data[at..end].copy_from_slice(bytes);
        Ok(())
    }

    pub fn read(&mut self, h: Handle, caller: u8) -> Result<Vec<u8>, MemError> {
        Ok(self.block_mut(h, caller)?.data.clone())
    }

    /// Revoca: azzera il blocco prima di liberarlo, così il riuso non espone dati altrui.
    pub fn revoke(&mut self, h: Handle, caller: u8) -> Result<(), MemError> {
        self.block_mut(h, caller)?;
        let slot = self.pool_mut(h.pool)[h.slot].take();
        if let Some(mut b) = slot {
            b.data.iter_mut().for_each(|x| *x = 0);
        }
        self.used[h.owner as usize] -= 1;
        Ok(())
    }
}
