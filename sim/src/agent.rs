use crate::{DenyReason, ProgramId};

/// Richiesta di un programma: quanti byte vuole.
#[derive(Clone, Copy, Debug)]
pub struct QuotaRequest {
    pub program: ProgramId,
    pub bytes: usize,
}

/// Risposta dell'agente. È solo una proposta: il monolite la verifica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proposal {
    Grant(usize),
    Deny(DenyReason),
}

/// L'agente risponde subito e non blocca mai il sistema.
pub trait Agent {
    fn propose(&mut self, req: &QuotaRequest, free_slots: usize) -> Proposal;
}

/// Agente di riferimento: concede la quota richiesta se c'è uno slot libero.
pub struct GreedyAgent;

impl Agent for GreedyAgent {
    fn propose(&mut self, req: &QuotaRequest, free_slots: usize) -> Proposal {
        if free_slots > 0 {
            Proposal::Grant(req.bytes)
        } else {
            Proposal::Deny(DenyReason::NoFreeSlot)
        }
    }
}
