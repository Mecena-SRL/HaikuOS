//! Visualizzatore foto: data una foto di W×H pixel calcola il rettangolo visibile e la scala.
//! Nessun buffer di pixel, solo dimensioni e pan/zoom. Schermo 240×160, zoom intero 1..4.

pub const SCHERMO_W: u16 = 240;
pub const SCHERMO_H: u16 = 160;
pub const ZOOM_MIN: u8 = 1;
pub const ZOOM_MAX: u8 = 4;

/// Rettangolo in pixel della foto (coordinate della foto, non dello schermo).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// Scala intera più grande (1..4) con cui la foto entra per intero nello schermo.
/// Se non entra nemmeno a 1x, vale 1 (si scorre con il pan).
pub fn scala_adatta(w: u16, h: u16) -> u8 {
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    for s in (ZOOM_MIN..=ZOOM_MAX).rev() {
        let s = s as u32;
        if w * s <= SCHERMO_W as u32 && h * s <= SCHERMO_H as u32 {
            return s as u8;
        }
    }
    ZOOM_MIN
}

fn fino_a(v: i32, max: u16) -> u16 {
    v.clamp(0, max as i32) as u16
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhotoView {
    w: u16,
    h: u16,
    zoom: u8,
    /// Angolo in alto a sinistra della vista, in pixel della foto.
    x: u16,
    y: u16,
}

impl PhotoView {
    /// Foto di `w`×`h` (le dimensioni minime sono 1×1), con zoom di adattamento e vista centrata.
    pub fn new(w: u16, h: u16) -> Self {
        let (w, h) = (w.max(1), h.max(1));
        let mut v = PhotoView { w, h, zoom: scala_adatta(w, h), x: 0, y: 0 };
        v.centra();
        v
    }

    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    pub fn dimensioni(&self) -> (u16, u16) {
        (self.w, self.h)
    }

    /// Dimensioni della parte di foto visibile a zoom corrente.
    fn vista(&self) -> (u16, u16) {
        let z = self.zoom as u16;
        ((SCHERMO_W / z).min(self.w), (SCHERMO_H / z).min(self.h))
    }

    /// Parte della foto che finisce sullo schermo.
    pub fn rettangolo(&self) -> Rect {
        let (w, h) = self.vista();
        Rect { x: self.x, y: self.y, w, h }
    }

    /// Angolo in alto a sinistra, sullo schermo, della parte visibile ingrandita.
    /// Non è zero se la foto è più piccola dello schermo: viene centrata.
    pub fn origine_schermo(&self) -> (u16, u16) {
        let (w, h) = self.vista();
        let z = self.zoom as u16;
        ((SCHERMO_W - w * z) / 2, (SCHERMO_H - h * z) / 2)
    }

    /// Vero se tutta la foto è visibile.
    pub fn entra_tutta(&self) -> bool {
        self.vista() == (self.w, self.h)
    }

    /// Sposta la vista di `dx`, `dy` pixel della foto (anche negativi), senza uscire dall'immagine.
    pub fn pan(&mut self, dx: i32, dy: i32) {
        let (w, h) = self.vista();
        self.x = fino_a(self.x as i32 + dx, self.w - w);
        self.y = fino_a(self.y as i32 + dy, self.h - h);
    }

    /// Zoom avanti di un passo (massimo 4). `false` se già al massimo.
    pub fn zoom_avanti(&mut self) -> bool {
        if self.zoom >= ZOOM_MAX {
            return false;
        }
        self.cambia_zoom(self.zoom + 1);
        true
    }

    /// Zoom indietro di un passo (minimo 1). `false` se già al minimo.
    pub fn zoom_indietro(&mut self) -> bool {
        if self.zoom <= ZOOM_MIN {
            return false;
        }
        self.cambia_zoom(self.zoom - 1);
        true
    }

    /// Torna allo zoom che fa entrare tutta la foto e la centra.
    pub fn adatta(&mut self) {
        self.zoom = scala_adatta(self.w, self.h);
        self.centra();
    }

    fn centra(&mut self) {
        let (w, h) = self.vista();
        self.x = (self.w - w) / 2;
        self.y = (self.h - h) / 2;
    }

    /// Cambia zoom mantenendo il centro della vista, poi riporta la vista dentro la foto.
    fn cambia_zoom(&mut self, nuovo: u8) {
        let (w, h) = self.vista();
        let cx = self.x as i32 + w as i32 / 2;
        let cy = self.y as i32 + h as i32 / 2;
        self.zoom = nuovo;
        let (nw, nh) = self.vista();
        self.x = fino_a(cx - nw as i32 / 2, self.w - nw);
        self.y = fino_a(cy - nh as i32 / 2, self.h - nh);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scala_adatta_sceglie_il_piu_grande_che_entra() {
        assert_eq!(super::scala_adatta(240, 160), 1);
        assert_eq!(super::scala_adatta(241, 10), 1);
        assert_eq!(super::scala_adatta(120, 80), 2);
        assert_eq!(super::scala_adatta(100, 80), 2);
        assert_eq!(super::scala_adatta(80, 53), 3); // 240 e 159 entrano; a 4x l'altezza è 212
        assert_eq!(super::scala_adatta(60, 40), 4);
        assert_eq!(super::scala_adatta(0, 0), 4); // trattata come 1x1
    }

    #[test]
    fn foto_piu_piccola_dello_schermo_entra_tutta_e_si_centra() {
        let v = super::PhotoView::new(100, 80);
        assert_eq!(v.zoom(), 2);
        assert!(v.entra_tutta());
        assert_eq!(v.rettangolo(), super::Rect { x: 0, y: 0, w: 100, h: 80 });
        assert_eq!(v.origine_schermo(), (20, 0)); // 200×160 centrato
    }

    #[test]
    fn foto_minuscola_usa_zoom_quattro_e_non_va_oltre() {
        let mut v = super::PhotoView::new(30, 20);
        assert_eq!(v.zoom(), 4);
        assert!(!v.zoom_avanti());
        assert_eq!(v.zoom(), 4);
        assert_eq!(v.origine_schermo(), (60, 40)); // 120×80 centrato
    }

    #[test]
    fn foto_grande_parte_centrata_a_zoom_uno() {
        let v = super::PhotoView::new(640, 480);
        assert_eq!(v.zoom(), 1);
        assert!(!v.entra_tutta());
        assert_eq!(v.rettangolo(), super::Rect { x: 200, y: 160, w: 240, h: 160 });
        assert_eq!(v.origine_schermo(), (0, 0));
    }

    #[test]
    fn zoom_avanti_mantiene_il_centro() {
        let mut v = super::PhotoView::new(640, 480);
        assert!(v.zoom_avanti());
        assert_eq!(v.zoom(), 2);
        // centro prima: (320, 240); a zoom 2 la vista è 120×80
        assert_eq!(v.rettangolo(), super::Rect { x: 260, y: 200, w: 120, h: 80 });
    }

    #[test]
    fn zoom_al_massimo_e_vista_minima() {
        let mut v = super::PhotoView::new(640, 480);
        assert!(v.zoom_avanti());
        assert!(v.zoom_avanti());
        assert!(v.zoom_avanti());
        assert_eq!(v.zoom(), 4);
        assert!(!v.zoom_avanti());
        assert_eq!(v.rettangolo().w, 60);
        assert_eq!(v.rettangolo().h, 40);
    }

    #[test]
    fn zoom_indietro_si_ferma_a_uno() {
        let mut v = super::PhotoView::new(640, 480);
        assert!(!v.zoom_indietro());
        assert!(v.zoom_avanti());
        assert!(v.zoom_indietro());
        assert_eq!(v.zoom(), 1);
        assert_eq!(v.rettangolo(), super::Rect { x: 200, y: 160, w: 240, h: 160 });
    }

    #[test]
    fn zoom_indietro_riporta_la_vista_dentro_la_foto() {
        let mut v = super::PhotoView::new(640, 480);
        v.pan(-10_000, -10_000); // angolo in alto a sinistra
        assert!(v.zoom_avanti());
        assert!(v.zoom_indietro());
        let r = v.rettangolo();
        assert_eq!((r.x, r.y), (0, 0));
        assert_eq!((r.w, r.h), (240, 160));
    }

    #[test]
    fn pan_si_ferma_ai_bordi() {
        let mut v = super::PhotoView::new(640, 480);
        v.pan(-10_000, -10_000);
        assert_eq!((v.rettangolo().x, v.rettangolo().y), (0, 0));
        v.pan(100_000, 100_000);
        assert_eq!((v.rettangolo().x, v.rettangolo().y), (400, 320)); // 640-240, 480-160
        v.pan(-7, -3);
        assert_eq!((v.rettangolo().x, v.rettangolo().y), (393, 317));
    }

    #[test]
    fn pan_a_zoom_massimo_usa_la_vista_piccola() {
        let mut v = super::PhotoView::new(640, 480);
        for _ in 0..3 {
            v.zoom_avanti();
        }
        v.pan(100_000, 100_000);
        let r = v.rettangolo();
        assert_eq!((r.x, r.y), (580, 440)); // 640-60, 480-40
        assert_eq!((r.w, r.h), (60, 40));
    }

    #[test]
    fn foto_esatta_dello_schermo_non_si_muove() {
        let mut v = super::PhotoView::new(240, 160);
        assert!(v.entra_tutta());
        v.pan(50, 50);
        assert_eq!((v.rettangolo().x, v.rettangolo().y), (0, 0));
        assert_eq!(v.origine_schermo(), (0, 0));
    }

    #[test]
    fn foto_larga_ma_bassa_pan_solo_in_orizzontale() {
        let mut v = super::PhotoView::new(480, 100);
        assert_eq!(v.zoom(), 1);
        v.pan(1000, 1000);
        let r = v.rettangolo();
        assert_eq!((r.x, r.y), (240, 0));
        assert_eq!((r.w, r.h), (240, 100));
        assert_eq!(v.origine_schermo(), (0, 30)); // 100 pixel centrati in 160
    }

    #[test]
    fn adatta_torna_alla_scala_intera_e_centra() {
        let mut v = super::PhotoView::new(640, 480);
        v.zoom_avanti();
        v.zoom_avanti();
        v.pan(-1000, -1000);
        v.adatta();
        assert_eq!(v.zoom(), 1);
        assert_eq!(v.rettangolo(), super::Rect { x: 200, y: 160, w: 240, h: 160 });
    }

    #[test]
    fn dimensioni_zero_diventano_uno() {
        let v = super::PhotoView::new(0, 0);
        assert_eq!(v.dimensioni(), (1, 1));
        assert_eq!(v.zoom(), 4);
        assert_eq!(v.rettangolo(), super::Rect { x: 0, y: 0, w: 1, h: 1 });
    }

    #[test]
    fn la_vista_sta_sempre_dentro_la_foto() {
        let dimensioni = [(1u16, 1u16), (240, 160), (241, 161), (7, 900), (900, 7), (640, 480)];
        for &(w, h) in &dimensioni {
            let mut v = super::PhotoView::new(w, h);
            for step in 0..8 {
                match step % 4 {
                    0 => { v.zoom_avanti(); }
                    1 => v.pan(37, -41),
                    2 => { v.zoom_indietro(); }
                    _ => v.pan(-500, 500),
                }
                let r = v.rettangolo();
                assert!(r.x as u32 + r.w as u32 <= w as u32, "{w}x{h} step {step}");
                assert!(r.y as u32 + r.h as u32 <= h as u32, "{w}x{h} step {step}");
                assert!(r.w as u32 * v.zoom() as u32 <= super::SCHERMO_W as u32);
                assert!(r.h as u32 * v.zoom() as u32 <= super::SCHERMO_H as u32);
            }
        }
    }
}
