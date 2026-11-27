//! Calcolatrice a quattro operazioni su interi con segno (solo `core`, testabile sull'host).

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

pub struct Calc {
    acc: i64,
    op: Option<Op>,
    cur: i64,
    typing: bool,
    error: bool,
}

impl Calc {
    pub const fn new() -> Self {
        Calc { acc: 0, op: None, cur: 0, typing: false, error: false }
    }

    /// Esegue l'operazione in sospeso (o prende `cur` se non ce n'è una). Overflow e divisione per zero danno errore.
    fn apply(&mut self) {
        let r = match self.op {
            None => Some(self.cur),
            Some(Op::Add) => self.acc.checked_add(self.cur),
            Some(Op::Sub) => self.acc.checked_sub(self.cur),
            Some(Op::Mul) => self.acc.checked_mul(self.cur),
            Some(Op::Div) => self.acc.checked_div(self.cur),
        };
        match r {
            Some(v) => {
                self.acc = v;
                self.cur = v;
            }
            None => self.error = true,
        }
    }

    pub fn digit(&mut self, d: u8) {
        if self.error {
            return;
        }
        if !self.typing {
            self.cur = 0;
            self.typing = true;
        }
        match self.cur.checked_mul(10).and_then(|v| v.checked_add(d as i64)) {
            Some(v) => self.cur = v,
            None => self.error = true,
        }
    }

    pub fn op(&mut self, o: Op) {
        if self.error {
            return;
        }
        self.apply();
        self.op = Some(o);
        self.typing = false;
    }

    pub fn equals(&mut self) {
        if self.error {
            return;
        }
        self.apply();
        self.op = None;
        self.typing = false;
    }

    /// Testo da mostrare: il numero corrente (con segno) oppure "ERR". Ritorna la lunghezza scritta in `out`.
    pub fn format(&self, out: &mut [u8; 24]) -> usize {
        if self.error {
            out[..3].copy_from_slice(b"ERR");
            return 3;
        }
        let mut tmp = [0u8; 20];
        let mut i = tmp.len();
        let mut n = self.cur.unsigned_abs();
        loop {
            i -= 1;
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        let mut len = 0;
        if self.cur < 0 {
            out[0] = b'-';
            len = 1;
        }
        for &b in &tmp[i..] {
            out[len] = b;
            len += 1;
        }
        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testo(c: &Calc) -> std::string::String {
        let mut b = [0u8; 24];
        let n = c.format(&mut b);
        std::string::String::from_utf8(b[..n].to_vec()).unwrap()
    }

    #[test]
    fn somma_e_prodotto() {
        let mut c = Calc::new();
        for d in [1, 2] {
            c.digit(d);
        }
        c.op(Op::Add);
        for d in [3, 0] {
            c.digit(d);
        }
        c.equals();
        assert_eq!(testo(&c), "42");
        c.digit(9);
        c.op(Op::Mul);
        c.digit(9);
        c.equals();
        assert_eq!(testo(&c), "81");
    }

    #[test]
    fn catena_di_operazioni() {
        let mut c = Calc::new();
        c.digit(1);
        c.digit(0);
        c.op(Op::Sub);
        c.digit(4);
        c.op(Op::Sub);
        c.digit(3);
        c.equals();
        assert_eq!(testo(&c), "3");
    }

    #[test]
    fn negativi_e_divisione_per_zero() {
        let mut c = Calc::new();
        c.digit(3);
        c.op(Op::Sub);
        c.digit(5);
        c.equals();
        assert_eq!(testo(&c), "-2");
        let mut d = Calc::new();
        d.digit(7);
        d.op(Op::Div);
        d.digit(0);
        d.equals();
        assert_eq!(testo(&d), "ERR");
    }

    #[test]
    fn overflow_da_errore() {
        let mut c = Calc::new();
        for _ in 0..19 {
            c.digit(9);
        }
        assert_eq!(testo(&c), "ERR");
    }
}
