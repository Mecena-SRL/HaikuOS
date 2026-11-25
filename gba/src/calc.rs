//! Calcolatrice: quattro operazioni con decimali in virgola fissa (2 cifre dopo la virgola).
//! Valori in centesimi (`i32`): 1,50 = 150. Logica pura, senza disegno.
//! Errori: divisione per zero e overflow. Dopo un errore resta `err` finché non si preme `clear`.
#![allow(dead_code)]

/// Cifre decimali fisse.
pub const SCALE: i32 = 100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Err {
    DivZero,
    Overflow,
}

pub struct Calc {
    /// Operando sinistro in attesa di un operatore.
    acc: i32,
    op: Option<Op>,
    /// Numero in inserimento, in centesimi.
    cur: i32,
    /// Vero dopo la virgola.
    point: bool,
    /// Cifre già inserite dopo la virgola (0..=2).
    frac: u8,
    /// Vero se il prossimo numero deve ricominciare da zero (dopo `=` o un operatore).
    fresh: bool,
    pub err: Option<Err>,
}

/// Applica `op` a due valori in centesimi. Moltiplicazione e divisione mantengono la scala.
pub fn apply(a: i32, op: Op, b: i32) -> Result<i32, Err> {
    let r: i64 = match op {
        Op::Add => a as i64 + b as i64,
        Op::Sub => a as i64 - b as i64,
        Op::Mul => a as i64 * b as i64 / SCALE as i64,
        Op::Div => {
            if b == 0 {
                return Err(Err::DivZero);
            }
            a as i64 * SCALE as i64 / b as i64
        }
    };
    if r > i32::MAX as i64 || r < i32::MIN as i64 {
        return Err(Err::Overflow);
    }
    Ok(r as i32)
}

impl Calc {
    pub const fn new() -> Self {
        Self { acc: 0, op: None, cur: 0, point: false, frac: 0, fresh: false, err: None }
    }

    /// Valore mostrato: il numero in inserimento, o il risultato.
    pub fn shown(&self) -> i32 {
        self.cur
    }

    /// Cifra 0..=9. Dopo la virgola accetta due cifre; oltre, ignora. Un overflow non cambia nulla.
    pub fn digit(&mut self, d: u8) {
        if self.err.is_some() || d > 9 {
            return;
        }
        if self.fresh {
            self.cur = 0;
            self.point = false;
            self.frac = 0;
            self.fresh = false;
        }
        if !self.point {
            if let Some(v) = self.cur.checked_mul(10).and_then(|v| v.checked_add(d as i32 * SCALE)) {
                self.cur = v;
            }
        } else if self.frac < 2 {
            self.frac += 1;
            let place = if self.frac == 1 { 10 } else { 1 };
            self.cur += d as i32 * place;
        }
    }

    /// Inserisce la virgola (una sola volta per numero).
    pub fn point(&mut self) {
        if self.err.is_some() || self.point {
            return;
        }
        if self.fresh {
            self.cur = 0;
            self.frac = 0;
            self.fresh = false;
        }
        self.point = true;
    }

    /// Sceglie l'operatore. Se c'è già un'operazione in attesa, la calcola prima.
    pub fn op(&mut self, o: Op) {
        if self.err.is_some() {
            return;
        }
        if self.op.is_some() && !self.fresh {
            if let Err(e) = self.eval() {
                self.err = Some(e);
                return;
            }
        }
        self.acc = self.cur;
        self.op = Some(o);
        self.fresh = true;
    }

    /// `=`: calcola l'operazione in attesa e mostra il risultato.
    pub fn equals(&mut self) {
        if self.err.is_some() {
            return;
        }
        if let Err(e) = self.eval() {
            self.err = Some(e);
        }
    }

    fn eval(&mut self) -> Result<(), Err> {
        if let Some(o) = self.op.take() {
            self.cur = apply(self.acc, o, self.cur)?;
            self.acc = self.cur;
            self.fresh = true;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

/// Scrive `v` (centesimi) come "-12.05" in `out`. Ritorna la lunghezza.
pub fn format(v: i32, out: &mut [u8; 12]) -> usize {
    let mut tmp = [0u8; 12];
    let mut n = 0;
    let neg = v < 0;
    let mut u = (v as i64).unsigned_abs();
    let frac = (u % SCALE as u64) as u8;
    u /= SCALE as u64;
    // Parte intera in cifre (almeno una) e poi parte decimale fissa a due cifre.
    loop {
        tmp[n] = b'0' + (u % 10) as u8;
        n += 1;
        u /= 10;
        if u == 0 {
            break;
        }
    }
    let mut len = 0;
    if neg {
        out[len] = b'-';
        len += 1;
    }
    for i in (0..n).rev() {
        out[len] = tmp[i];
        len += 1;
    }
    out[len] = b'.';
    out[len + 1] = b'0' + frac / 10;
    out[len + 2] = b'0' + frac % 10;
    len + 3
}
