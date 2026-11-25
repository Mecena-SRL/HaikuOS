//! Mappa: griglia 32×32 di celle (un bit per cella: libera o ostacolo).
//! Cursore con collisione e percorso minimo con BFS su array fissi, senza heap.
//! Il BFS usa circa 3 KB di stack: da verificare sul GBA.

pub const LATO: usize = 32;
const CELLE: usize = LATO * LATO;

/// Su, destra, giù, sinistra (x, y).
const DIREZIONI: [(i8, i8); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Griglia statica: una riga per `u32`, bit x a 1 = ostacolo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mappa {
    righe: [u32; LATO],
}

impl Mappa {
    pub const fn vuota() -> Self {
        Mappa { righe: [0; LATO] }
    }

    /// Mappa da un array di righe (bit x a 1 = ostacolo).
    pub const fn da_array(righe: [u32; LATO]) -> Self {
        Mappa { righe }
    }

    /// Mappa generata dal seed con un LCG: circa un ostacolo ogni 4 celle.
    /// Stesso seed, stessa mappa. Gli angoli (0,0) e (31,31) restano liberi.
    pub fn da_seed(seme: u32) -> Self {
        let mut s = seme;
        let mut m = Mappa::vuota();
        for y in 0..LATO {
            for x in 0..LATO {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                if (s >> 16) % 4 == 0 {
                    m.imposta(x, y, true);
                }
            }
        }
        m.imposta(0, 0, false);
        m.imposta(LATO - 1, LATO - 1, false);
        m
    }

    /// Vero se la cella è un ostacolo. Fuori griglia vale come ostacolo.
    pub fn ostacolo(&self, x: usize, y: usize) -> bool {
        if x >= LATO || y >= LATO {
            return true;
        }
        (self.righe[y] & (1u32 << x)) != 0
    }

    /// Vero se la cella è dentro la griglia e non è un ostacolo.
    pub fn libera(&self, x: usize, y: usize) -> bool {
        x < LATO && y < LATO && !self.ostacolo(x, y)
    }

    /// Segna la cella come ostacolo o libera. Fuori griglia non fa nulla.
    pub fn imposta(&mut self, x: usize, y: usize, ostacolo: bool) {
        if x >= LATO || y >= LATO {
            return;
        }
        if ostacolo {
            self.righe[y] |= 1u32 << x;
        } else {
            self.righe[y] &= !(1u32 << x);
        }
    }
}

/// Cursore sulla griglia: non entra mai in un ostacolo né esce dalla mappa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursore {
    x: usize,
    y: usize,
}

impl Cursore {
    pub const fn new(x: usize, y: usize) -> Self {
        let x = if x < LATO { x } else { LATO - 1 };
        let y = if y < LATO { y } else { LATO - 1 };
        Cursore { x, y }
    }

    pub fn pos(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Muove di (dx, dy) celle. Restituisce `false` se la cella di arrivo è bloccata.
    pub fn muovi(&mut self, mappa: &Mappa, dx: i8, dy: i8) -> bool {
        let nx = self.x as isize + dx as isize;
        let ny = self.y as isize + dy as isize;
        if nx < 0 || ny < 0 {
            return false;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        if mappa.libera(nx, ny) {
            self.x = nx;
            self.y = ny;
            true
        } else {
            false
        }
    }
}

/// Esito di una ricerca di percorso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Percorso {
    /// Numero di passi (0 se partenza e arrivo coincidono).
    pub lunghezza: usize,
    /// Spostamento (dx, dy) del primo passo; `None` se non ci si muove.
    pub primo_passo: Option<(i8, i8)>,
}

/// Percorso minimo da `da` ad `a` su 4 direzioni (BFS).
/// `None` se non esiste, o se un estremo è un ostacolo o fuori griglia.
pub fn percorso(mappa: &Mappa, da: (usize, usize), a: (usize, usize)) -> Option<Percorso> {
    if !mappa.libera(da.0, da.1) || !mappa.libera(a.0, a.1) {
        return None;
    }
    let partenza = da.1 * LATO + da.0;
    let arrivo = a.1 * LATO + a.0;
    if partenza == arrivo {
        return Some(Percorso { lunghezza: 0, primo_passo: None });
    }

    // primo[c]: 0 = non visitata; 1..=4 = direzione del primo passo + 1; 5 = partenza.
    let mut primo = [0u8; CELLE];
    let mut coda = [0u16; CELLE];
    let mut testa = 0usize;
    let mut fine = 1usize;
    coda[0] = partenza as u16;
    primo[partenza] = 5;

    let mut livello = 0usize;
    let mut fine_livello = fine;
    while testa < fine {
        if testa == fine_livello {
            livello += 1;
            fine_livello = fine;
        }
        let c = coda[testa] as usize;
        testa += 1;
        if c == arrivo {
            return Some(Percorso {
                lunghezza: livello,
                primo_passo: Some(DIREZIONI[(primo[c] - 1) as usize]),
            });
        }
        for (d, &(dx, dy)) in DIREZIONI.iter().enumerate() {
            let x = (c % LATO) as isize + dx as isize;
            let y = (c / LATO) as isize + dy as isize;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            if !mappa.libera(x, y) {
                continue;
            }
            let n = y * LATO + x;
            if primo[n] != 0 {
                continue;
            }
            primo[n] = if c == partenza { d as u8 + 1 } else { primo[c] };
            coda[fine] = n as u16;
            fine += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Muro verticale in colonna `x` da riga 0 a `fino` (esclusa).
    fn muro(x: usize, fino: usize) -> Mappa {
        let mut m = Mappa::vuota();
        for y in 0..fino {
            m.imposta(x, y, true);
        }
        m
    }

    #[test]
    fn percorso_semplice_in_linea() {
        let m = Mappa::vuota();
        let p = percorso(&m, (0, 0), (3, 0)).unwrap();
        assert_eq!(p.lunghezza, 3);
        assert_eq!(p.primo_passo, Some((1, 0)));
    }

    #[test]
    fn percorso_diagonale_ha_lunghezza_manhattan() {
        let m = Mappa::vuota();
        let p = percorso(&m, (0, 0), (2, 2)).unwrap();
        assert_eq!(p.lunghezza, 4);
        assert_eq!(p.primo_passo, Some((1, 0)));
    }

    #[test]
    fn partenza_uguale_arrivo() {
        let m = Mappa::vuota();
        let p = percorso(&m, (7, 7), (7, 7)).unwrap();
        assert_eq!(p.lunghezza, 0);
        assert_eq!(p.primo_passo, None);
    }

    #[test]
    fn nessun_percorso_con_muro_chiuso() {
        let m = muro(5, LATO);
        assert_eq!(percorso(&m, (0, 0), (10, 0)), None);
    }

    #[test]
    fn ostacoli_costringono_a_deviare() {
        // Muro in colonna 5 fino alla riga 30: passaggio solo sull'ultima riga.
        let m = muro(5, LATO - 1);
        let p = percorso(&m, (0, 0), (10, 0)).unwrap();
        assert_eq!(p.lunghezza, 5 + 31 + 5 + 31);
        assert_eq!(p.primo_passo, Some((1, 0)));
    }

    #[test]
    fn arrivo_o_partenza_su_ostacolo() {
        let mut m = Mappa::vuota();
        m.imposta(4, 4, true);
        assert_eq!(percorso(&m, (4, 4), (0, 0)), None);
        assert_eq!(percorso(&m, (0, 0), (4, 4)), None);
    }

    #[test]
    fn bordi_e_fuori_griglia() {
        let m = Mappa::vuota();
        assert!(!m.libera(LATO, 0));
        assert!(!m.libera(0, LATO));
        assert!(m.libera(LATO - 1, LATO - 1));
        assert_eq!(percorso(&m, (0, 0), (LATO, 0)), None);
        assert_eq!(percorso(&m, (0, 0), (LATO - 1, LATO - 1)).unwrap().lunghezza, 62);
    }

    #[test]
    fn cursore_non_esce_dalla_mappa() {
        let m = Mappa::vuota();
        let mut c = Cursore::new(0, 0);
        assert!(!c.muovi(&m, -1, 0));
        assert!(!c.muovi(&m, 0, -1));
        assert_eq!(c.pos(), (0, 0));
        let mut c = Cursore::new(LATO - 1, LATO - 1);
        assert!(!c.muovi(&m, 1, 0));
        assert!(!c.muovi(&m, 0, 1));
        assert!(c.muovi(&m, -1, -1));
        assert_eq!(c.pos(), (LATO - 2, LATO - 2));
    }

    #[test]
    fn cursore_bloccato_dagli_ostacoli() {
        let mut m = Mappa::vuota();
        m.imposta(1, 0, true);
        let mut c = Cursore::new(0, 0);
        assert!(!c.muovi(&m, 1, 0));
        assert_eq!(c.pos(), (0, 0));
        assert!(c.muovi(&m, 0, 1));
        assert_eq!(c.pos(), (0, 1));
    }

    #[test]
    fn imposta_e_leggi_celle() {
        let mut m = Mappa::vuota();
        m.imposta(31, 31, true);
        assert!(m.ostacolo(31, 31));
        assert!(!m.ostacolo(30, 31));
        m.imposta(31, 31, false);
        assert!(m.libera(31, 31));
    }

    #[test]
    fn da_array_usa_i_bit_delle_righe() {
        let mut righe = [0u32; LATO];
        righe[2] = 1 << 3;
        let m = Mappa::da_array(righe);
        assert!(m.ostacolo(3, 2));
        assert!(!m.ostacolo(2, 2));
    }

    #[test]
    fn da_seed_e_deterministica_e_lascia_liberi_gli_angoli() {
        let a = Mappa::da_seed(1234);
        assert_eq!(a, Mappa::da_seed(1234));
        assert_ne!(a, Mappa::da_seed(4321));
        assert!(a.libera(0, 0));
        assert!(a.libera(LATO - 1, LATO - 1));
    }
}
