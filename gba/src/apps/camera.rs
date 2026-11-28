//! Fotocamera: macchina a stati (Spenta, Anteprima, Scatto, Salvata, Errore) guidata da eventi.
//! Nessun accesso all'hardware. Il GBA non ha fotocamera: il comportamento reale è da verificare su macchina potente.

/// Foto salvabili: oltre questo numero il salvataggio dà errore "memoria piena".
pub const LIMITE: u8 = 16;

/// Avviso da mostrare nella voce: il GBA non ha fotocamera.
pub const AVVISO: &str = "Il GBA non ha fotocamera: da verificare su macchina potente.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Causa {
    MemoriaPiena,
    Hardware,
}

impl Causa {
    pub fn testo(&self) -> &'static str {
        match self {
            Causa::MemoriaPiena => "memoria piena",
            Causa::Hardware => "errore fotocamera",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Spenta,
    Anteprima,
    Scatto,
    Salvata,
    Errore(Causa),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evento {
    Accendi,
    Scatta,
    Salva,
    Spegni,
    Errore(Causa),
}

pub struct Camera {
    stato: Stato,
    salvate: u8,
}

impl Camera {
    pub const fn new() -> Self {
        Camera { stato: Stato::Spenta, salvate: 0 }
    }

    pub fn stato(&self) -> Stato {
        self.stato
    }

    /// Foto salvate finora, da 0 a `LIMITE`.
    pub fn salvate(&self) -> u8 {
        self.salvate
    }

    pub fn piena(&self) -> bool {
        self.salvate >= LIMITE
    }

    /// Applica un evento. Ritorna `false` (e non cambia nulla) se l'evento non è valido nello stato attuale.
    /// `Spegni` ed `Errore` sono sempre validi.
    pub fn evento(&mut self, e: Evento) -> bool {
        match (self.stato, e) {
            (_, Evento::Spegni) => {
                self.stato = Stato::Spenta;
                true
            }
            (_, Evento::Errore(c)) => {
                self.stato = Stato::Errore(c);
                true
            }
            (Stato::Spenta, Evento::Accendi) | (Stato::Errore(_), Evento::Accendi) => {
                self.stato = Stato::Anteprima;
                true
            }
            (Stato::Anteprima, Evento::Scatta) | (Stato::Salvata, Evento::Scatta) => {
                self.stato = Stato::Scatto;
                true
            }
            (Stato::Scatto, Evento::Salva) => {
                if self.piena() {
                    self.stato = Stato::Errore(Causa::MemoriaPiena);
                } else {
                    self.salvate += 1;
                    self.stato = Stato::Salvata;
                }
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ciclo_accendi_scatta_salva() {
        let mut c = super::Camera::new();
        assert_eq!(c.stato(), super::Stato::Spenta);
        assert!(c.evento(super::Evento::Accendi));
        assert_eq!(c.stato(), super::Stato::Anteprima);
        assert!(c.evento(super::Evento::Scatta));
        assert_eq!(c.stato(), super::Stato::Scatto);
        assert!(c.evento(super::Evento::Salva));
        assert_eq!(c.stato(), super::Stato::Salvata);
        assert_eq!(c.salvate(), 1);
    }

    #[test]
    fn evento_non_valido_e_ignorato() {
        let mut c = super::Camera::new();
        assert!(!c.evento(super::Evento::Scatta)); // spenta
        assert!(!c.evento(super::Evento::Salva)); // nessuno scatto in corso
        assert_eq!(c.stato(), super::Stato::Spenta);
        assert_eq!(c.salvate(), 0);
        assert!(c.evento(super::Evento::Accendi));
        assert!(!c.evento(super::Evento::Accendi)); // già accesa
        assert!(!c.evento(super::Evento::Salva)); // anteprima, non scatto
        assert_eq!(c.stato(), super::Stato::Anteprima);
    }

    #[test]
    fn spegni_e_sempre_valido() {
        let mut c = super::Camera::new();
        assert!(c.evento(super::Evento::Spegni)); // già spenta
        assert_eq!(c.stato(), super::Stato::Spenta);
        c.evento(super::Evento::Accendi);
        c.evento(super::Evento::Scatta);
        assert!(c.evento(super::Evento::Spegni)); // a metà scatto
        assert_eq!(c.stato(), super::Stato::Spenta);
    }

    #[test]
    fn errore_hardware_e_recuperabile_con_accendi() {
        let mut c = super::Camera::new();
        c.evento(super::Evento::Accendi);
        assert!(c.evento(super::Evento::Errore(super::Causa::Hardware)));
        assert_eq!(c.stato(), super::Stato::Errore(super::Causa::Hardware));
        assert!(!c.evento(super::Evento::Scatta)); // in errore non si scatta
        assert!(c.evento(super::Evento::Accendi));
        assert_eq!(c.stato(), super::Stato::Anteprima);
        assert_eq!(super::Causa::Hardware.testo(), "errore fotocamera");
    }

    #[test]
    fn salvata_permette_nuovo_scatto() {
        let mut c = super::Camera::new();
        c.evento(super::Evento::Accendi);
        c.evento(super::Evento::Scatta);
        c.evento(super::Evento::Salva);
        assert!(c.evento(super::Evento::Scatta));
        assert_eq!(c.stato(), super::Stato::Scatto);
    }

    #[test]
    fn limite_di_sedici_foto() {
        let mut c = super::Camera::new();
        c.evento(super::Evento::Accendi);
        for n in 1..=16u8 {
            assert!(c.evento(super::Evento::Scatta));
            assert!(c.evento(super::Evento::Salva));
            assert_eq!(c.salvate(), n);
        }
        assert!(c.piena());
        assert!(c.evento(super::Evento::Scatta));
        assert!(c.evento(super::Evento::Salva)); // tentativo oltre il limite
        assert_eq!(c.stato(), super::Stato::Errore(super::Causa::MemoriaPiena));
        assert_eq!(c.salvate(), 16);
        assert_eq!(super::Causa::MemoriaPiena.testo(), "memoria piena");
    }

    #[test]
    fn memoria_piena_resta_con_riaccensione() {
        let mut c = super::Camera::new();
        c.evento(super::Evento::Accendi);
        for _ in 0..16 {
            c.evento(super::Evento::Scatta);
            c.evento(super::Evento::Salva);
        }
        c.evento(super::Evento::Scatta);
        c.evento(super::Evento::Salva);
        c.evento(super::Evento::Accendi);
        assert_eq!(c.stato(), super::Stato::Anteprima);
        c.evento(super::Evento::Scatta);
        c.evento(super::Evento::Salva);
        assert_eq!(c.stato(), super::Stato::Errore(super::Causa::MemoriaPiena));
        assert_eq!(c.salvate(), 16);
    }

    #[test]
    fn avviso_segnala_la_verifica_su_hardware() {
        assert!(super::AVVISO.contains("da verificare su macchina potente"));
        assert_eq!(super::LIMITE, 16);
    }
}
