//! Animazioni e UI reattiva: tween con easing, transizioni tra schermate, budget per frame.
//! Tutto a interi (ms, permille, µs): no_std, nessuna allocazione nel percorso di disegno.
use crate::layout::Rect;
use crate::palette::Tok;
use crate::render::Canvas;

/// Un frame a 60 Hz, in µs.
pub const FRAME_US: u32 = 16_667;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ease {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    OutCubic,
}

/// Easing su permille: `t` in 0..=1000 (clampato), risultato in 0..=1000.
pub fn ease(e: Ease, t: i32) -> i32 {
    let t = t.clamp(0, 1000);
    let u = 1000 - t;
    match e {
        Ease::Linear => t,
        Ease::InQuad => t * t / 1000,
        Ease::OutQuad => 1000 - u * u / 1000,
        Ease::InOutQuad if t < 500 => 2 * t * t / 1000,
        Ease::InOutQuad => 1000 - 2 * u * u / 1000,
        Ease::OutCubic => 1000 - u * u * u / 1_000_000,
    }
}

/// Interpolazione da `from` a `to` fra `start` e `start + dur` (ms); fuori intervallo vale il bordo.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    pub from: i32,
    pub to: i32,
    pub start: u32,
    pub dur: u32,
    pub ease: Ease,
}

impl Tween {
    pub fn value(&self, now: u32) -> i32 {
        if self.dur == 0 || now >= self.start.saturating_add(self.dur) {
            return self.to;
        }
        let t = if now <= self.start { 0 } else { ((now - self.start) as u64 * 1000 / self.dur as u64) as i32 };
        let p = ease(self.ease, t) as i64;
        self.from + ((self.to - self.from) as i64 * p / 1000) as i32
    }

    pub fn finished(&self, now: u32) -> bool {
        now >= self.start.saturating_add(self.dur)
    }
}

/// Valore reattivo: `set` parte dal valore visualizzato in quel momento, così un cambio
/// di bersaglio a metà animazione non salta.
#[derive(Clone, Copy, Debug)]
pub struct Reactive {
    tw: Tween,
}

impl Reactive {
    pub const fn new(v: i32) -> Reactive {
        Reactive { tw: Tween { from: v, to: v, start: 0, dur: 0, ease: Ease::Linear } }
    }

    pub fn get(&self, now: u32) -> i32 {
        self.tw.value(now)
    }

    /// Nuovo bersaglio. Se è già il bersaglio corrente non riavvia l'animazione.
    pub fn set(&mut self, to: i32, now: u32, dur: u32, ease: Ease) {
        if to == self.tw.to {
            return;
        }
        let from = self.get(now);
        self.tw = Tween { from, to, start: now, dur, ease };
    }

    pub fn settled(&self, now: u32) -> bool {
        self.tw.finished(now)
    }
}

/// Budget di frame: registra il tempo di ogni frame (µs) e conta quelli oltre `target_us`.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub target_us: u32,
    pub frames: u32,
    pub over: u32,
    pub last_us: u32,
    pub max_us: u32,
    total_us: u64,
}

impl Budget {
    pub const fn new(target_us: u32) -> Budget {
        Budget { target_us, frames: 0, over: 0, last_us: 0, max_us: 0, total_us: 0 }
    }

    pub fn record(&mut self, us: u32) {
        self.frames += 1;
        self.last_us = us;
        self.max_us = self.max_us.max(us);
        self.total_us += us as u64;
        if us > self.target_us {
            self.over += 1;
        }
    }

    pub fn avg_us(&self) -> u32 {
        if self.frames == 0 { 0 } else { (self.total_us / self.frames as u64) as u32 }
    }

    /// µs che restano nel budget dopo l'ultimo frame (0 se è stato sforato).
    pub fn headroom_us(&self) -> u32 {
        self.target_us.saturating_sub(self.last_us)
    }
}

/// Transizione orizzontale: la schermata vecchia scivola via a sinistra, la nuova entra da destra.
#[derive(Clone, Copy, Debug)]
pub struct Slide {
    tw: Tween,
    w: i32,
}

impl Slide {
    pub fn start(w: i32, now: u32, dur: u32) -> Slide {
        Slide { tw: Tween { from: 0, to: 1000, start: now, dur, ease: Ease::OutCubic }, w }
    }

    pub fn finished(&self, now: u32) -> bool {
        self.tw.finished(now)
    }

    /// Spostamenti in pixel `(vecchia, nuova)` da passare a `Shifted`.
    pub fn offsets(&self, now: u32) -> (i32, i32) {
        let x = self.w * self.tw.value(now) / 1000;
        (-x, self.w - x)
    }
}

/// Vista di un `Canvas` spostata di `(dx, dy)`: una schermata si disegna con gli stessi metodi di sempre.
pub struct Shifted<'a, C: Canvas> {
    pub inner: &'a mut C,
    pub dx: i32,
    pub dy: i32,
}

impl<C: Canvas> Shifted<'_, C> {
    fn sh(&self, r: Rect) -> Rect {
        Rect::new(r.x + self.dx, r.y + self.dy, r.w, r.h)
    }
}

impl<C: Canvas> Canvas for Shifted<'_, C> {
    fn fill(&mut self, r: Rect, t: Tok) {
        let r = self.sh(r);
        self.inner.fill(r, t);
    }
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        self.inner.text(x + self.dx, y + self.dy, s, t);
    }
    fn rounded(&self) -> bool {
        self.inner.rounded()
    }
    fn rich(&self) -> bool {
        self.inner.rich()
    }
    fn icon(&mut self, cell: Rect, label: &str, app: u8) {
        let cell = self.sh(cell);
        self.inner.icon(cell, label, app);
    }
    fn rect_rounded(&mut self, r: Rect, radius: i32, t: Tok) {
        let r = self.sh(r);
        self.inner.rect_rounded(r, radius, t);
    }
    fn icon_vec(&mut self, cell: Rect, rects: &[(i32, i32, i32, i32)], t: Tok) {
        let cell = self.sh(cell);
        self.inner.icon_vec(cell, rects, t);
    }
    fn menu(&mut self, r: Rect, rows: &[&str], sel: usize) {
        let r = self.sh(r);
        self.inner.menu(r, rows, sel);
    }
    fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, w: i32, t: Tok) {
        self.inner.line(x1 + self.dx, y1 + self.dy, x2 + self.dx, y2 + self.dy, w, t);
    }
}
