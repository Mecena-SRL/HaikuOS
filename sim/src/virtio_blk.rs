//! Driver blocchi in stile virtio-blk (idea Linux/FreeBSD, riscritta dalla specifica virtio):
//! richieste IN/OUT su settore da 512 byte, coda fissa, stato per richiesta. Nessun hardware reale.
//! Regole: docs/kernel-study.md, riga 21 (virtio prima, AHCI dopo).

use crate::geom::Block;
use crate::resource::Queue;

pub const SECTOR: usize = 512;
pub const QUEUE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    IoErr,
}

#[derive(Clone, Copy, Debug)]
pub struct Req {
    pub id: u16,
    pub op: Op,
    pub sector: u64,
    pub data: [u8; SECTOR],
}

#[derive(Clone, Copy, Debug)]
pub struct Done {
    pub id: u16,
    pub status: Status,
    /// Dati letti (solo per `Read`).
    pub data: [u8; SECTOR],
}

impl Default for Req {
    fn default() -> Self {
        Self { id: 0, op: Op::Read, sector: 0, data: [0; SECTOR] }
    }
}
impl Default for Done {
    fn default() -> Self {
        Self { id: 0, status: Status::Ok, data: [0; SECTOR] }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BlkError {
    Full,
}

pub struct VirtioBlk {
    reqs: Queue<Req, QUEUE>,
    done: Queue<Done, QUEUE>,
    next_id: u16,
}

impl VirtioBlk {
    pub fn new() -> Self {
        Self { reqs: Queue::new(QUEUE), done: Queue::new(QUEUE), next_id: 1 }
    }

    /// Lato programma: accoda una richiesta e ritorna il suo id (mai zero).
    pub fn submit(&mut self, op: Op, sector: u64, data: [u8; SECTOR]) -> Result<u16, BlkError> {
        let id = self.next_id;
        self.reqs.push(Req { id, op, sector, data }).map_err(|_| BlkError::Full)?;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        Ok(id)
    }

    /// Lato dispositivo: esegue le richieste in ordine su `disk` (blocchi da 512 byte).
    /// Un errore di I/O finisce nello stato, non ferma la coda.
    pub fn process<B: Block>(&mut self, disk: &mut B) -> usize {
        let mut n = 0;
        while let Some(r) = self.reqs.pop() {
            let mut d = Done { id: r.id, status: Status::Ok, data: [0; SECTOR] };
            let res = match r.op {
                Op::Read => disk.read(r.sector as usize, &mut d.data),
                Op::Write => disk.write(r.sector as usize, &r.data),
            };
            if res.is_err() {
                d.status = Status::IoErr;
            }
            // Coda di completamento piena: la richiesta torna in coda per il prossimo giro.
            if self.done.push(d).is_err() {
                let _ = self.reqs.push(r);
                break;
            }
            n += 1;
        }
        n
    }

    /// Lato programma: ritira il prossimo completamento, in ordine.
    pub fn complete(&mut self) -> Option<Done> {
        self.done.pop()
    }
}
