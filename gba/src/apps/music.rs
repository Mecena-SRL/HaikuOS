//! Libreria musicale: fino a 16 brani e 4 playlist da 8 brani.
//! Solo logica di riproduzione (play, pausa, giro, ripeti, mescola), senza audio.
//! Audio su GBA: da verificare.

pub const MAX_BRANI: usize = 16;
pub const MAX_PLAYLIST: usize = 4;
pub const BRANI_PER_PLAYLIST: usize = 8;
pub const TITOLO_MAX: usize = 16;

/// Brano: titolo fino a 16 byte e durata in secondi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brano {
    titolo: [u8; TITOLO_MAX],
    len: u8,
    pub durata: u16,
}

const BRANO_VUOTO: Brano = Brano { titolo: [0; TITOLO_MAX], len: 0, durata: 0 };

impl Brano {
    /// Crea un brano; `None` se il titolo supera 16 byte.
    pub fn new(titolo: &[u8], durata: u16) -> Option<Self> {
        if titolo.len() > TITOLO_MAX {
            return None;
        }
        let mut t = [0u8; TITOLO_MAX];
        t[..titolo.len()].copy_from_slice(titolo);
        Some(Brano { titolo: t, len: titolo.len() as u8, durata })
    }

    pub fn titolo(&self) -> &[u8] {
        &self.titolo[..self.len as usize]
    }
}

/// Playlist: indici di brani della libreria (fino a 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Playlist {
    brani: [u8; BRANI_PER_PLAYLIST],
    len: u8,
}

impl Playlist {
    pub const fn vuota() -> Self {
        Playlist { brani: [0; BRANI_PER_PLAYLIST], len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Indice in libreria del brano in posizione `pos` della playlist.
    pub fn brano(&self, pos: usize) -> Option<usize> {
        if pos < self.len() {
            Some(self.brani[pos] as usize)
        } else {
            None
        }
    }
}

/// Libreria: brani e playlist, senza heap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Libreria {
    brani: [Brano; MAX_BRANI],
    len: usize,
    playlist: [Playlist; MAX_PLAYLIST],
}

impl Libreria {
    pub const fn new() -> Self {
        Libreria {
            brani: [BRANO_VUOTO; MAX_BRANI],
            len: 0,
            playlist: [Playlist::vuota(); MAX_PLAYLIST],
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn brano(&self, i: usize) -> Option<&Brano> {
        if i < self.len {
            Some(&self.brani[i])
        } else {
            None
        }
    }

    pub fn playlist(&self, i: usize) -> Option<&Playlist> {
        self.playlist.get(i)
    }

    /// Aggiunge un brano e ne restituisce l'indice. `None` se la libreria è piena
    /// o il titolo è troppo lungo.
    pub fn aggiungi(&mut self, titolo: &[u8], durata: u16) -> Option<usize> {
        if self.len >= MAX_BRANI {
            return None;
        }
        let b = Brano::new(titolo, durata)?;
        self.brani[self.len] = b;
        self.len += 1;
        Some(self.len - 1)
    }

    /// Aggiunge il brano `brano` (indice in libreria) alla playlist `pl`.
    /// `false` se la playlist o il brano non esistono, o la playlist è piena.
    pub fn aggiungi_a_playlist(&mut self, pl: usize, brano: usize) -> bool {
        if pl >= MAX_PLAYLIST || brano >= self.len {
            return false;
        }
        let p = &mut self.playlist[pl];
        if p.len() >= BRANI_PER_PLAYLIST {
            return false;
        }
        p.brani[p.len()] = brano as u8;
        p.len += 1;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Fermo,
    InRiproduzione,
    InPausa,
}

/// Stato di riproduzione di una playlist.
///
/// Il giro è sempre attivo per successivo e precedente (la playlist è circolare).
/// `ripeti` decide cosa succede quando un brano finisce sull'ultimo della playlist:
/// con `ripeti` si ricomincia, senza si ferma.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lettore {
    stato: Stato,
    /// Playlist in uso.
    pl: usize,
    /// Posizione in `ordine` (0..lunghezza della playlist).
    k: usize,
    /// Ordine di riproduzione: posizioni nella playlist (identità o mescolate).
    ordine: [u8; BRANI_PER_PLAYLIST],
    /// Secondi trascorsi nel brano corrente.
    trascorsi: u16,
    ripeti: bool,
    mescola: bool,
    /// Stato del generatore pseudo-casuale (LCG).
    seme: u32,
}

impl Lettore {
    /// Il seme rende la mescola deterministica: stesso seme, stesso ordine.
    pub const fn new(seme: u32) -> Self {
        Lettore {
            stato: Stato::Fermo,
            pl: 0,
            k: 0,
            ordine: [0, 1, 2, 3, 4, 5, 6, 7],
            trascorsi: 0,
            ripeti: false,
            mescola: false,
            seme,
        }
    }

    fn prossimo(&mut self) -> u32 {
        self.seme = self.seme.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.seme >> 16
    }

    fn lunghezza(&self, lib: &Libreria) -> usize {
        lib.playlist(self.pl).map_or(0, |p| p.len())
    }

    /// Ricostruisce l'ordine per la playlist corrente (Fisher-Yates se `mescola`).
    /// Con `corrente` = `Some(c)` la posizione `k` segue il brano di posizione `c`;
    /// con `None` si torna all'inizio dell'ordine.
    fn ricostruisci(&mut self, lib: &Libreria, corrente: Option<usize>) {
        let n = self.lunghezza(lib);
        for (i, v) in self.ordine.iter_mut().enumerate() {
            *v = i as u8;
        }
        if self.mescola {
            let mut i = n;
            while i > 1 {
                i -= 1;
                let j = self.prossimo() as usize % (i + 1);
                self.ordine.swap(i, j);
            }
        }
        self.k = match corrente {
            Some(c) => self.ordine[..n].iter().position(|&v| v as usize == c).unwrap_or(0),
            None => 0,
        };
    }

    /// Sceglie la playlist `pl` e si posiziona all'inizio, senza avviare.
    /// `false` se la playlist non esiste o è vuota.
    pub fn seleziona(&mut self, lib: &Libreria, pl: usize) -> bool {
        match lib.playlist(pl) {
            Some(p) if !p.is_empty() => {}
            _ => return false,
        }
        self.pl = pl;
        self.stato = Stato::Fermo;
        self.trascorsi = 0;
        self.ricostruisci(lib, None);
        true
    }

    /// Avvia la riproduzione, o riprende dalla pausa.
    pub fn play(&mut self, lib: &Libreria) -> bool {
        if self.lunghezza(lib) == 0 {
            return false;
        }
        match self.stato {
            Stato::InRiproduzione => false,
            Stato::InPausa => {
                self.stato = Stato::InRiproduzione;
                true
            }
            Stato::Fermo => {
                self.stato = Stato::InRiproduzione;
                self.trascorsi = 0;
                true
            }
        }
    }

    pub fn pausa(&mut self) -> bool {
        if self.stato == Stato::InRiproduzione {
            self.stato = Stato::InPausa;
            true
        } else {
            false
        }
    }

    /// Ferma e riporta il brano corrente all'inizio.
    pub fn ferma(&mut self) {
        self.stato = Stato::Fermo;
        self.trascorsi = 0;
    }

    /// Brano successivo, con giro: dall'ultimo si torna al primo.
    pub fn successivo(&mut self, lib: &Libreria) {
        let n = self.lunghezza(lib);
        if n == 0 {
            return;
        }
        self.k = (self.k + 1) % n;
        self.trascorsi = 0;
    }

    /// Brano precedente, con giro: dal primo si va all'ultimo.
    pub fn precedente(&mut self, lib: &Libreria) {
        let n = self.lunghezza(lib);
        if n == 0 {
            return;
        }
        self.k = (self.k + n - 1) % n;
        self.trascorsi = 0;
    }

    /// Avanza di `secondi` il brano in riproduzione. A fine brano passa al successivo.
    /// Sull'ultimo brano: con `ripeti` ricomincia (rimescolando se `mescola`),
    /// altrimenti si ferma.
    pub fn tick(&mut self, lib: &Libreria, secondi: u16) {
        if self.stato != Stato::InRiproduzione {
            return;
        }
        self.trascorsi = self.trascorsi.saturating_add(secondi);
        let durata = self
            .brano_corrente(lib)
            .and_then(|i| lib.brano(i))
            .map_or(0, |b| b.durata);
        if self.trascorsi < durata {
            return;
        }
        self.trascorsi = 0;
        let n = self.lunghezza(lib);
        if self.k + 1 < n {
            self.k += 1;
        } else if self.ripeti {
            if self.mescola {
                self.ricostruisci(lib, None);
            } else {
                self.k = 0;
            }
        } else {
            self.stato = Stato::Fermo;
            self.k = 0;
        }
    }

    pub fn set_ripeti(&mut self, valore: bool) {
        self.ripeti = valore;
    }

    /// Attiva o disattiva la mescola. Il brano corrente resta lo stesso.
    pub fn set_mescola(&mut self, lib: &Libreria, valore: bool) {
        if self.mescola == valore {
            return;
        }
        let corrente = self.ordine[self.k] as usize;
        self.mescola = valore;
        self.ricostruisci(lib, Some(corrente));
    }

    pub fn stato(&self) -> Stato {
        self.stato
    }

    pub fn ripeti(&self) -> bool {
        self.ripeti
    }

    pub fn mescola(&self) -> bool {
        self.mescola
    }

    /// Indice in libreria del brano selezionato (in riproduzione o no).
    pub fn brano_corrente(&self, lib: &Libreria) -> Option<usize> {
        lib.playlist(self.pl)?.brano(self.ordine[self.k] as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn libreria(durate: &[u16]) -> Libreria {
        let mut lib = Libreria::new();
        for (i, &d) in durate.iter().enumerate() {
            let nome = [b'T', b'0' + i as u8];
            lib.aggiungi(&nome, d).unwrap();
        }
        lib
    }

    #[test]
    fn titolo_fino_a_16_byte() {
        let mut lib = Libreria::new();
        assert!(lib.aggiungi(b"0123456789ABCDEF", 10).is_some());
        assert!(lib.aggiungi(b"0123456789ABCDEFG", 10).is_none());
        assert_eq!(lib.brano(0).unwrap().titolo(), &b"0123456789ABCDEF"[..]);
    }

    #[test]
    fn libreria_piena_a_16_brani() {
        let mut lib = Libreria::new();
        for i in 0..MAX_BRANI {
            assert_eq!(lib.aggiungi(b"x", i as u16), Some(i));
        }
        assert_eq!(lib.aggiungi(b"x", 1), None);
        assert_eq!(lib.len(), MAX_BRANI);
    }

    #[test]
    fn playlist_max_8_brani_e_4_playlist() {
        let mut lib = libreria(&[1]);
        for _ in 0..BRANI_PER_PLAYLIST {
            assert!(lib.aggiungi_a_playlist(0, 0));
        }
        assert!(!lib.aggiungi_a_playlist(0, 0));
        assert!(!lib.aggiungi_a_playlist(MAX_PLAYLIST, 0));
        assert!(!lib.aggiungi_a_playlist(1, 5));
        assert_eq!(lib.playlist(0).unwrap().len(), 8);
        assert_eq!(lib.playlist(0).unwrap().brano(7), Some(0));
    }

    #[test]
    fn play_pausa_ferma() {
        let mut lib = libreria(&[10, 10]);
        let mut l = Lettore::new(1);
        assert!(!l.seleziona(&lib, 0), "playlist 0 non ancora creata");
        lib.aggiungi_a_playlist(0, 0);
        lib.aggiungi_a_playlist(0, 1);
        assert!(l.seleziona(&lib, 0));
        assert_eq!(l.stato(), Stato::Fermo);
        assert!(l.play(&lib));
        assert_eq!(l.stato(), Stato::InRiproduzione);
        assert!(!l.play(&lib));
        assert!(l.pausa());
        assert_eq!(l.stato(), Stato::InPausa);
        assert!(l.play(&lib));
        assert_eq!(l.stato(), Stato::InRiproduzione);
        l.ferma();
        assert_eq!(l.stato(), Stato::Fermo);
    }

    #[test]
    fn play_senza_brani_non_parte() {
        let lib = libreria(&[10]);
        let mut l = Lettore::new(1);
        assert!(!l.seleziona(&lib, 0));
        assert!(!l.play(&lib));
        assert_eq!(l.stato(), Stato::Fermo);
    }

    fn lib_tre_brani() -> Libreria {
        let mut lib = libreria(&[10, 20, 30]);
        for i in 0..3 {
            lib.aggiungi_a_playlist(0, i);
        }
        lib
    }

    #[test]
    fn successivo_e_precedente_con_giro() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(1);
        l.seleziona(&lib, 0);
        assert_eq!(l.brano_corrente(&lib), Some(0));
        l.successivo(&lib);
        assert_eq!(l.brano_corrente(&lib), Some(1));
        l.successivo(&lib);
        assert_eq!(l.brano_corrente(&lib), Some(2));
        l.successivo(&lib);
        assert_eq!(l.brano_corrente(&lib), Some(0));
        l.precedente(&lib);
        assert_eq!(l.brano_corrente(&lib), Some(2));
    }

    #[test]
    fn tick_passa_al_brano_successivo() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(1);
        l.seleziona(&lib, 0);
        l.play(&lib);
        l.tick(&lib, 9);
        assert_eq!(l.brano_corrente(&lib), Some(0));
        l.tick(&lib, 1);
        assert_eq!(l.brano_corrente(&lib), Some(1));
        l.tick(&lib, 25);
        assert_eq!(l.brano_corrente(&lib), Some(2));
        assert_eq!(l.stato(), Stato::InRiproduzione);
    }

    #[test]
    fn tick_in_pausa_non_avanza() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(1);
        l.seleziona(&lib, 0);
        l.play(&lib);
        l.pausa();
        l.tick(&lib, 100);
        assert_eq!(l.brano_corrente(&lib), Some(0));
    }

    #[test]
    fn fine_playlist_senza_ripeti_ferma() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(1);
        l.seleziona(&lib, 0);
        l.play(&lib);
        l.tick(&lib, 10);
        l.tick(&lib, 20);
        l.tick(&lib, 30);
        assert_eq!(l.stato(), Stato::Fermo);
        assert_eq!(l.brano_corrente(&lib), Some(0));
    }

    #[test]
    fn ripeti_ricomincia_la_playlist() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(1);
        l.set_ripeti(true);
        assert!(l.ripeti());
        l.seleziona(&lib, 0);
        l.play(&lib);
        l.tick(&lib, 10);
        l.tick(&lib, 20);
        l.tick(&lib, 30);
        assert_eq!(l.stato(), Stato::InRiproduzione);
        assert_eq!(l.brano_corrente(&lib), Some(0));
    }

    #[test]
    fn mescola_e_deterministica_e_una_permutazione() {
        let mut lib = libreria(&[5, 5, 5, 5, 5, 5, 5, 5]);
        for i in 0..8 {
            lib.aggiungi_a_playlist(0, i);
        }
        let raccogli = |seme: u32| {
            let mut l = Lettore::new(seme);
            l.seleziona(&lib, 0);
            l.set_mescola(&lib, true);
            let mut seq = [0usize; 8];
            for s in seq.iter_mut() {
                *s = l.brano_corrente(&lib).unwrap();
                l.successivo(&lib);
            }
            seq
        };
        let a = raccogli(42);
        assert_eq!(a, raccogli(42));
        assert_ne!(a, raccogli(7));
        let mut visti = [false; 8];
        for &v in a.iter() {
            assert!(!visti[v], "brano ripetuto nella mescola");
            visti[v] = true;
        }
        assert!(visti.iter().all(|&v| v));
    }

    #[test]
    fn mescola_mantiene_il_brano_corrente() {
        let lib = lib_tre_brani();
        let mut l = Lettore::new(9);
        l.seleziona(&lib, 0);
        l.successivo(&lib);
        l.successivo(&lib);
        assert_eq!(l.brano_corrente(&lib), Some(2));
        l.set_mescola(&lib, true);
        assert!(l.mescola());
        assert_eq!(l.brano_corrente(&lib), Some(2));
        l.set_mescola(&lib, false);
        assert_eq!(l.brano_corrente(&lib), Some(2));
    }
}
