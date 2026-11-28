//! Passo fisso: la logica gira a frequenza fissa, indipendente dal tempo di disegno.

pub struct FixedStep {
    tick_us: u64,
    max_ticks: u64,
    acc_us: u64,
}

impl FixedStep {
    /// `hz` tick al secondo; al massimo `max_ticks` per chiamata (il resto si scarta).
    pub const fn new(hz: u32, max_ticks: u32) -> Self {
        FixedStep { tick_us: 1_000_000 / hz as u64, max_ticks: max_ticks as u64, acc_us: 0 }
    }

    /// Aggiunge `elapsed_us` e restituisce quanti tick di logica eseguire.
    pub fn advance(&mut self, elapsed_us: u64) -> u32 {
        self.acc_us += elapsed_us;
        let due = self.acc_us / self.tick_us;
        let n = due.min(self.max_ticks);
        self.acc_us = if due > n { 0 } else { self.acc_us - n * self.tick_us };
        n as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_hz_accumulates() {
        let mut s = FixedStep::new(60, 4);
        assert_eq!(s.advance(10_000), 0);
        assert_eq!(s.advance(10_000), 1); // 20 ms -> 1 tick (16.6 ms), resto 3.3 ms
        assert_eq!(s.advance(13_000), 0); // 16.3 ms in tutto, sotto 16.67 ms
    }

    #[test]
    fn cap_drops_the_backlog() {
        let mut s = FixedStep::new(60, 4);
        assert_eq!(s.advance(1_000_000), 4);
        assert_eq!(s.advance(0), 0);
    }
}
