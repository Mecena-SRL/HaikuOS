//! Gestione processi sull'host: tabella fissa, scheduling a priorità fisse con budget.
//! Nessuna allocazione dinamica dopo `new`. Il budget limita il tempo di CPU per turno,
//! così un programma non può monopolizzare il sistema (vedi docs/process-management.md).
//! Priorità: valore più basso = priorità più alta. Stessa priorità: round-robin.

use crate::{ProgramId, MAX_PROGRAMS};

/// Numero massimo di processi: uguale al limite del monolite (`MAX_PROGRAMS`).
pub const MAX_PROCS: usize = MAX_PROGRAMS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ready,
    Running,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnError {
    Full,
    Duplicate,
    /// Budget zero: il processo non riceverebbe mai CPU.
    ZeroBudget,
}

#[derive(Clone, Copy, Debug)]
struct Proc {
    id: ProgramId,
    prio: u8,
    budget: u32,
    left: u32,
    state: State,
    /// Istante in cui il processo ha ceduto la CPU l'ultima volta (per il round-robin).
    last_run: u64,
}

pub struct Scheduler {
    procs: Vec<Proc>,
    running: Option<usize>,
    clock: u64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self { procs: Vec::with_capacity(MAX_PROCS), running: None, clock: 0 }
    }

    /// Crea un processo pronto. Non ruba la CPU a un processo a priorità uguale o più alta.
    pub fn spawn(&mut self, id: ProgramId, prio: u8, budget: u32) -> Result<(), SpawnError> {
        if budget == 0 {
            return Err(SpawnError::ZeroBudget);
        }
        if self.procs.len() >= MAX_PROCS {
            return Err(SpawnError::Full);
        }
        if self.index_of(id).is_some() {
            return Err(SpawnError::Duplicate);
        }
        self.procs.push(Proc { id, prio, budget, left: budget, state: State::Ready, last_run: 0 });
        self.preempt_if_lower_prio_running();
        Ok(())
    }

    /// Avanza di un tick. Ritorna il processo che ha usato la CPU in questo tick (None = idle).
    pub fn tick(&mut self) -> Option<ProgramId> {
        self.clock += 1;
        if self.running.is_none() {
            self.dispatch();
        }
        let i = self.running?;
        let ran = self.procs[i].id;
        self.procs[i].left -= 1;
        if self.procs[i].left == 0 {
            // Budget esaurito: il processo torna in coda, dietro i pari priorità.
            self.procs[i].state = State::Ready;
            self.procs[i].last_run = self.clock;
            self.running = None;
        }
        Some(ran)
    }

    /// Mette il processo in attesa (es. messaggio). Se era in esecuzione, la CPU passa ad altri.
    pub fn block(&mut self, id: ProgramId) -> bool {
        let Some(i) = self.index_of(id) else { return false };
        self.procs[i].state = State::Blocked;
        if self.running == Some(i) {
            self.running = None;
        }
        true
    }

    /// Risveglia un processo bloccato. Se ha priorità più alta, prende la CPU subito.
    pub fn wake(&mut self, id: ProgramId) -> bool {
        let Some(i) = self.index_of(id) else { return false };
        if self.procs[i].state != State::Blocked {
            return false;
        }
        self.procs[i].state = State::Ready;
        self.procs[i].left = self.procs[i].budget;
        self.preempt_if_lower_prio_running();
        true
    }

    /// Rimuove il processo dalla tabella: libera lo slot.
    pub fn exit(&mut self, id: ProgramId) -> bool {
        let Some(i) = self.index_of(id) else { return false };
        if self.running == Some(i) {
            self.running = None;
        }
        self.procs.swap_remove(i);
        // Dopo swap_remove l'indice in esecuzione può essere cambiato: si ricalcola.
        self.running = self.procs.iter().position(|p| p.state == State::Running);
        true
    }

    pub fn state_of(&self, id: ProgramId) -> Option<State> {
        self.index_of(id).map(|i| self.procs[i].state)
    }

    pub fn count(&self) -> usize {
        self.procs.len()
    }

    fn index_of(&self, id: ProgramId) -> Option<usize> {
        self.procs.iter().position(|p| p.id == id)
    }

    /// Sceglie il pronto con priorità più alta; a parità, quello che ha aspettato di più.
    fn dispatch(&mut self) {
        let next = self
            .procs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.state == State::Ready)
            .min_by_key(|(_, p)| (p.prio, p.last_run))
            .map(|(i, _)| i);
        if let Some(i) = next {
            self.procs[i].state = State::Running;
            self.procs[i].left = self.procs[i].budget;
            self.procs[i].last_run = self.clock;
            self.running = Some(i);
        }
    }

    /// Se un pronto ha priorità più alta di chi è in esecuzione, quest'ultimo torna pronto.
    fn preempt_if_lower_prio_running(&mut self) {
        let Some(r) = self.running else {
            return;
        };
        let run_prio = self.procs[r].prio;
        let higher_ready = self
            .procs
            .iter()
            .any(|p| p.state == State::Ready && p.prio < run_prio);
        if higher_ready {
            self.procs[r].state = State::Ready;
            self.procs[r].last_run = self.clock;
            self.running = None;
        }
    }
}
