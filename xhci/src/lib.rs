//! Anelli TRB di xHCI (docs/kernel-study.md, riga 11): comandi ed eventi con bit di ciclo.
//! Pure strutture e regole; nessun accesso ai registri MMIO. Il kernel le mapperà sulla memoria DMA.
#![cfg_attr(not(test), no_std)]

/// Transfer Request Block: 16 byte, come da specifica xHCI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Trb {
    pub param: u64,
    pub status: u32,
    pub control: u32,
}

pub const TYPE_LINK: u8 = 6;
pub const TYPE_ENABLE_SLOT: u8 = 9;
pub const TYPE_NOOP_CMD: u8 = 23;
pub const TYPE_CMD_COMPLETION: u8 = 33;
pub const TYPE_PORT_STATUS: u8 = 34;

const CYCLE: u32 = 1;
const TOGGLE: u32 = 1 << 1;

impl Trb {
    pub const fn new(param: u64, status: u32, kind: u8) -> Self {
        Self { param, status, control: (kind as u32) << 10 }
    }
    pub fn kind(&self) -> u8 {
        ((self.control >> 10) & 0x3F) as u8
    }
    pub fn cycle(&self) -> bool {
        self.control & CYCLE != 0
    }
}

/// Anello di comandi (produttore). L'ultimo slot è un Link che riporta a `base`
/// e inverte il bit di ciclo. Con N slot ce ne sono N-1 utili.
pub struct CmdRing<const N: usize> {
    trbs: [Trb; N],
    base: u64,
    idx: usize,
    cycle: bool,
}

impl<const N: usize> CmdRing<N> {
    pub fn new(base: u64) -> Self {
        assert!(N >= 2);
        Self { trbs: [Trb::default(); N], base, idx: 0, cycle: true }
    }

    /// Scrive un TRB con il bit di ciclo corrente. Ritorna lo slot usato.
    pub fn push(&mut self, mut t: Trb) -> usize {
        t.control = (t.control & !CYCLE) | (self.cycle as u32);
        let slot = self.idx;
        self.trbs[slot] = t;
        self.idx += 1;
        if self.idx == N - 1 {
            let mut link = Trb::new(self.base, 0, TYPE_LINK);
            link.control |= TOGGLE | (self.cycle as u32);
            self.trbs[N - 1] = link;
            self.idx = 0;
            self.cycle = !self.cycle;
        }
        slot
    }

    pub fn trbs(&self) -> &[Trb; N] {
        &self.trbs
    }
}

/// Legge il prossimo evento da un anello di eventi scritto dall'hardware.
/// Valido se il bit di ciclo coincide con `ccs`; al giro completo `ccs` si inverte.
pub fn next_event(ring: &[Trb], idx: &mut usize, ccs: &mut bool) -> Option<Trb> {
    let t = ring[*idx];
    if t.cycle() != *ccs {
        return None;
    }
    *idx += 1;
    if *idx == ring.len() {
        *idx = 0;
        *ccs = !*ccs;
    }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trb_tipo_e_ciclo() {
        let t = Trb::new(0, 0, TYPE_NOOP_CMD);
        assert_eq!(t.kind(), 23);
        assert!(!t.cycle());
    }

    #[test]
    fn comandi_e_link_con_ciclo_invertito() {
        let mut r: CmdRing<4> = CmdRing::new(0x1000);
        assert_eq!(r.push(Trb::new(0, 0, TYPE_ENABLE_SLOT)), 0);
        assert_eq!(r.push(Trb::new(0, 0, TYPE_NOOP_CMD)), 1);
        // Con N=4 il Link finisce in slot 3: restano 3 slot utili.
        assert_eq!(r.push(Trb::new(0, 0, TYPE_NOOP_CMD)), 2);
        let t = r.trbs();
        assert!(t[0].cycle() && t[1].cycle() && t[2].cycle());
        // Link in slot N-1 con ciclo ancora 1 e bit Toggle.
        assert_eq!(t[3].kind(), TYPE_LINK);
        assert_eq!(t[3].param, 0x1000);
        assert!(t[3].cycle() && t[3].control & TOGGLE != 0);
        // Dopo il link, il giro successivo usa ciclo 0.
        assert_eq!(r.push(Trb::new(0, 0, TYPE_NOOP_CMD)), 0);
        assert!(!r.trbs()[0].cycle());
    }

    #[test]
    fn eventi_segnati_dal_ciclo() {
        // Anello di eventi di 4 slot: l'hardware scrive con ciclo 1 al primo giro.
        let mut ring = [Trb::default(); 4];
        ring[0] = Trb::new(7, 1, TYPE_CMD_COMPLETION);
        ring[0].control |= CYCLE;
        ring[1] = Trb::new(8, 1, TYPE_PORT_STATUS);
        ring[1].control |= CYCLE;
        let (mut idx, mut ccs) = (0usize, true);
        assert_eq!(next_event(&ring, &mut idx, &mut ccs).map(|t| t.param), Some(7));
        assert_eq!(next_event(&ring, &mut idx, &mut ccs).map(|t| t.kind()), Some(TYPE_PORT_STATUS));
        // Slot 2 non ancora scritto (ciclo 0): nessun evento.
        assert_eq!(next_event(&ring, &mut idx, &mut ccs), None);
        assert_eq!(idx, 2);
    }

    #[test]
    fn giro_completo_inverte_ccs() {
        let mut ring = [Trb::default(); 2];
        ring[0].control |= CYCLE;
        ring[1].control |= CYCLE;
        let (mut idx, mut ccs) = (0usize, true);
        assert!(next_event(&ring, &mut idx, &mut ccs).is_some());
        assert!(next_event(&ring, &mut idx, &mut ccs).is_some());
        // Dopo il giro, ccs è 0: gli slot con ciclo 1 non sono più validi.
        assert!(!ccs && idx == 0);
        assert_eq!(next_event(&ring, &mut idx, &mut ccs), None);
    }
}
