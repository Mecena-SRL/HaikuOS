//! Modello schermo (kernel-study righe 2–4): oggetti buffer con handle, coda di flip al vsync,
//! piani e output. Tutto a dimensione fissa, senza allocazione. Il monolite possiede i buffer;
//! i programmi vedono solo handle opachi.
use crate::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Full,
    BadHandle,
    Busy,
}

/// Handle opaco: indice e generazione. Un handle vecchio, dopo il riuso dello slot, è rifiutato.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Buffer {
    pub w: u32,
    pub h: u32,
    /// Byte per riga.
    pub pitch: u32,
}

#[derive(Clone, Copy)]
struct Slot {
    gen: u16,
    refs: u16,
    buf: Option<Buffer>,
}

/// Tabella di buffer con conteggio riferimenti. Un buffer è libero quando `refs` torna a 0.
pub struct Buffers<const N: usize> {
    slots: [Slot; N],
}

impl<const N: usize> Buffers<N> {
    pub const fn new() -> Self {
        Buffers { slots: [Slot { gen: 0, refs: 0, buf: None }; N] }
    }

    /// Crea un buffer con un riferimento.
    pub fn alloc(&mut self, buf: Buffer) -> Result<Handle, Error> {
        let (i, s) = self.slots.iter_mut().enumerate().find(|(_, s)| s.buf.is_none()).ok_or(Error::Full)?;
        s.buf = Some(buf);
        s.refs = 1;
        Ok(Handle(((s.gen as u32) << 16) | i as u32))
    }

    fn slot(&mut self, h: Handle) -> Result<&mut Slot, Error> {
        let (i, gen) = ((h.0 & 0xFFFF) as usize, (h.0 >> 16) as u16);
        match self.slots.get_mut(i) {
            Some(s) if s.gen == gen && s.buf.is_some() => Ok(s),
            _ => Err(Error::BadHandle),
        }
    }

    pub fn get(&mut self, h: Handle) -> Result<Buffer, Error> {
        self.slot(h).map(|s| s.buf.unwrap())
    }

    /// Aggiunge un riferimento (es. il compositore che tiene la finestra).
    pub fn retain(&mut self, h: Handle) -> Result<(), Error> {
        self.slot(h)?.refs += 1;
        Ok(())
    }

    /// Toglie un riferimento. `Ok(true)` se il buffer è stato liberato.
    pub fn release(&mut self, h: Handle) -> Result<bool, Error> {
        let s = self.slot(h)?;
        s.refs -= 1;
        if s.refs == 0 {
            s.buf = None;
            s.gen = s.gen.wrapping_add(1);
            return Ok(true);
        }
        Ok(false)
    }
}

/// Coda di flip con un solo slot in attesa: il buffer cambia solo al vsync.
#[derive(Default)]
pub struct Flip {
    pending: Option<Handle>,
    shown: Option<Handle>,
}

impl Flip {
    pub const fn new() -> Self {
        Flip { pending: None, shown: None }
    }

    /// Chiede il flip del buffer `h` al prossimo vsync. `Busy` se un flip è già in attesa.
    pub fn submit(&mut self, h: Handle) -> Result<(), Error> {
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        self.pending = Some(h);
        Ok(())
    }

    /// Chiamato al vsync: applica il flip in attesa e restituisce il buffer che non è più mostrato.
    pub fn vblank(&mut self) -> Option<Handle> {
        let next = self.pending.take()?;
        self.shown.replace(next)
    }

    pub fn shown(&self) -> Option<Handle> {
        self.shown
    }
}

/// Un piano: quale buffer, dove va sull'output. Più piani sono ammessi; per ora se ne usa uno.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plane {
    pub src: Option<Handle>,
    pub dst: Rect,
}

/// Piani e output. `N` piani fissi.
pub struct Pipe<const N: usize> {
    pub out_w: i32,
    pub out_h: i32,
    planes: [Plane; N],
}

impl<const N: usize> Pipe<N> {
    pub const fn new(out_w: i32, out_h: i32) -> Self {
        Pipe { out_w, out_h, planes: [Plane { src: None, dst: Rect::new(0, 0, 0, 0) }; N] }
    }

    pub fn set(&mut self, i: usize, src: Option<Handle>, dst: Rect) -> Result<(), Error> {
        let p = self.planes.get_mut(i).ok_or(Error::BadHandle)?;
        *p = Plane { src, dst };
        Ok(())
    }

    pub fn plane(&self, i: usize) -> Option<Plane> {
        self.planes.get(i).copied()
    }
}
