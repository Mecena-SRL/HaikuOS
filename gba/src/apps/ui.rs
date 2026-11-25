//! Stato delle app nel corpo della finestra (una istanza nel desktop) e tasti per ciascuna.
//! La logica sta nei moduli accanto (passwords, dictionary, ...); qui solo la sessione.
//! Le app solo hardware (fotocamera, memo, musica, messaggi) non hanno stato: mostrano "DA VERIFICARE".

use crate::apps::calendar::{Calendario, Data, Mese, Ora};
use crate::apps::dictionary::Sfoglia;
use crate::apps::maps::{Cursore, Mappa};
use crate::apps::passwords::Archivio;
use crate::apps::photo_view::PhotoView;
use crate::apps::photos::Photos;
use crate::desktop::{App, KEY_A, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_SELECT, KEY_UP};

/// Passo di spostamento della vista foto, in pixel della foto.
const PASSO_PAN: i32 = 4;

pub struct AppsUi {
    pub pass: Archivio,
    /// Segreto visibile (A nella finestra password).
    pub mostra: bool,
    pub dict: Sfoglia,
    pub cal: Calendario,
    pub mese: Mese,
    pub foto: Photos,
    pub vista: PhotoView,
    pub mappa: Mappa,
    pub cursore: Cursore,
}

impl AppsUi {
    /// Dati di prova: una password, un evento, tre foto, una mappa di seme fisso.
    pub fn new() -> Self {
        let mut pass = Archivio::new();
        let _ = pass.aggiungi(b"MAIL", b"ANNA", b"DEMO");
        let mut cal = Calendario::new();
        if let (Some(d), Some(o)) = (Data::new(2026, 11, 25), Ora::new(10, 0)) {
            cal.aggiungi(b"TELEFONA ANNA", d, o);
        }
        let mut foto = Photos::new();
        foto.aggiungi(b"GATTO", 64, 48, 0);
        foto.aggiungi(b"CASA", 96, 64, 1);
        foto.aggiungi(b"MARE", 48, 48, 2);
        Self {
            pass,
            mostra: false,
            dict: Sfoglia::tutte(),
            cal,
            mese: Mese { anno: 2026, mese: 11 },
            foto,
            vista: PhotoView::new(96, 64),
            mappa: Mappa::da_seed(7),
            cursore: Cursore::new(0, 0),
        }
    }

    /// Tasti dell'app `app`. B chiude la finestra e lo fa il desktop.
    /// Ritorna true se lo stato è cambiato (la finestra va ridisegnata).
    pub fn key(&mut self, app: App, pressed: u16) -> bool {
        let croce = KEY_UP | KEY_DOWN | KEY_LEFT | KEY_RIGHT;
        match app {
            App::Passwords => {
                if pressed & KEY_UP != 0 {
                    self.pass.su();
                }
                if pressed & KEY_DOWN != 0 {
                    self.pass.giu();
                }
                if pressed & KEY_A != 0 {
                    self.mostra = !self.mostra;
                }
                pressed & (croce | KEY_A) != 0
            }
            App::Dictionary => {
                let mut cambiato = false;
                if pressed & KEY_UP != 0 {
                    cambiato |= self.dict.su();
                }
                if pressed & KEY_DOWN != 0 {
                    cambiato |= self.dict.giu();
                }
                cambiato
            }
            App::Calendar => {
                if pressed & KEY_LEFT != 0 {
                    self.mese = self.mese.precedente();
                }
                if pressed & KEY_RIGHT != 0 {
                    self.mese = self.mese.successivo();
                }
                pressed & (KEY_LEFT | KEY_RIGHT) != 0
            }
            App::Photos => {
                if pressed & KEY_UP != 0 {
                    self.foto.su();
                }
                if pressed & KEY_DOWN != 0 {
                    self.foto.giu();
                }
                if pressed & KEY_LEFT != 0 {
                    self.foto.pagina_prec();
                }
                if pressed & KEY_RIGHT != 0 {
                    self.foto.pagina_succ();
                }
                pressed & croce != 0
            }
            App::PhotoView => {
                if pressed & KEY_UP != 0 {
                    self.vista.pan(0, -PASSO_PAN);
                }
                if pressed & KEY_DOWN != 0 {
                    self.vista.pan(0, PASSO_PAN);
                }
                if pressed & KEY_LEFT != 0 {
                    self.vista.pan(-PASSO_PAN, 0);
                }
                if pressed & KEY_RIGHT != 0 {
                    self.vista.pan(PASSO_PAN, 0);
                }
                if pressed & KEY_A != 0 {
                    self.vista.zoom_avanti();
                }
                if pressed & KEY_SELECT != 0 {
                    self.vista.zoom_indietro();
                }
                pressed & (croce | KEY_A | KEY_SELECT) != 0
            }
            App::Maps => {
                let (mut dx, mut dy) = (0i8, 0i8);
                if pressed & KEY_UP != 0 {
                    dy = -1;
                }
                if pressed & KEY_DOWN != 0 {
                    dy = 1;
                }
                if pressed & KEY_LEFT != 0 {
                    dx = -1;
                }
                if pressed & KEY_RIGHT != 0 {
                    dx = 1;
                }
                self.cursore.muovi(&self.mappa, dx, dy)
            }
            _ => false,
        }
    }
}
