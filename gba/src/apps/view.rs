//! Disegno delle app di sistema nel corpo della finestra (`x0`, `y0` = origine del corpo, 106×76).
//! Colori dalla palette del tema: `p[0]` testo, `p[1]` corpo, `p[2]` evidenza.

use crate::apps::calendar::griglia;
use crate::apps::maps::LATO;
use crate::apps::passwords::MAX_CAMPO;
use crate::apps::ui::AppsUi;
use crate::apps::{clock::Clock, notes::Notes, settings::Settings};
use crate::apps::timer_esercizi::Fase;
use crate::apps::testo_segnaposto;
use crate::font::{self, ADVANCE};

const BODY_W: usize = 106;
const BODY_H: usize = 76;
const COLS: usize = 24;
const ROWS: usize = 10;
const LINE: usize = 6;
const TOP: usize = 10;

fn fill(fb: &mut [u16], w: usize, h: usize, x: usize, y: usize, rw: usize, rh: usize, c: u16) {
    for yy in y..(y + rh).min(h) {
        for xx in x..(x + rw).min(w) {
            fb[yy * w + xx] = c;
        }
    }
}

fn digit(n: usize) -> u8 {
    b'0' + (n % 10) as u8
}

fn body(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, p: [u16; 4]) {
    fill(fb, w, h, x0, y0, BODY_W, BODY_H, p[1]);
}

/// Note: intestazione "NOTA n DI m", poi il testo a capo, con l'ultima riga in fondo.
pub fn notes(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, n: &Notes, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    if n.count() == 0 {
        font::draw(fb, w, h, x0 + 2, y0 + 2, b"NESSUNA NOTA", p[0]);
        return;
    }
    let head = [b'N', b'O', b'T', b'A', b' ', digit(n.selected() + 1), b' ', b'D', b'I', b' ', digit(n.count())];
    font::draw(fb, w, h, x0 + 2, y0 + 2, &head, p[0]);

    let text = n.testo();
    let len = text.len();
    let start = (len / COLS + 1).saturating_sub(ROWS);
    for i in 0..=len {
        let (c, color) = if i < len { (text[i], p[0]) } else { (n.pending(), p[2]) };
        let line = i / COLS;
        if line < start {
            continue;
        }
        let x = x0 + 2 + (i % COLS) * ADVANCE;
        let y = y0 + TOP + (line - start) * LINE;
        font::draw(fb, w, h, x, y, &[c], color);
    }
}

/// Orologio: ora grande al centro.
pub fn clock(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, c: &Clock, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    font::draw(fb, w, h, x0 + 43, y0 + 30, &c.testo(), p[0]);
}

/// Impostazioni: numero del tema, quattro campioni della palette, nome del sistema.
pub fn settings(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, s: &Settings, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let head = [b'T', b'E', b'M', b'A', b' ', digit(s.theme() + 1)];
    font::draw(fb, w, h, x0 + 2, y0 + 2, &head, p[0]);
    let pal = s.palette();
    for (i, &c) in pal.iter().enumerate() {
        fill(fb, w, h, x0 + 3 + i * 24, y0 + 17, 22, 16, p[0]);
        fill(fb, w, h, x0 + 4 + i * 24, y0 + 18, 20, 14, c);
    }
    font::draw(fb, w, h, x0 + 2, y0 + 42, b"NOME", p[0]);
    font::draw(fb, w, h, x0 + 2, y0 + 52, s.nome(), p[0]);
}

/// Due cifre (n < 100).
fn due(n: usize) -> [u8; 2] {
    [digit(n / 10), digit(n)]
}

/// "NOME aa DI bb", con numeri a due cifre. Il nome è al più 12 byte.
fn intestazione(fb: &mut [u16], w: usize, h: usize, x: usize, y: usize, nome: &[u8], a: usize, b: usize, c: u16) {
    let mut s = [b' '; 24];
    let mut k = 0;
    for &ch in nome.iter().take(12) {
        s[k] = ch;
        k += 1;
    }
    k += 1;
    s[k..k + 2].copy_from_slice(&due(a));
    k += 3;
    s[k..k + 2].copy_from_slice(b"DI");
    k += 3;
    s[k..k + 2].copy_from_slice(&due(b));
    k += 2;
    font::draw(fb, w, h, x, y, &s[..k], c);
}

/// Testo a capo per parole: `colonne` caratteri per riga, al più `righe` righe.
fn testo_a_capo(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, s: &[u8], colonne: usize, righe: usize, c: u16) {
    let (mut col, mut riga) = (0usize, 0usize);
    for parola in s.split(|&b| b == b' ').filter(|p| !p.is_empty()) {
        let n = parola.len().min(colonne);
        if col > 0 && col + 1 + n > colonne {
            col = 0;
            riga += 1;
        } else if col > 0 {
            col += 1;
        }
        if riga >= righe {
            return;
        }
        for &ch in &parola[..n] {
            font::draw(fb, w, h, x0 + col * ADVANCE, y0 + riga * LINE, &[ch], c);
            col += 1;
        }
    }
}

/// Password: una voce alla volta. Il segreto è nascosto finché A non lo mostra.
pub fn passwords(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    if let Some(m) = a.cassa.modifica {
        let titolo: &[u8] = if m.indice.is_some() { b"MODIFICA VOCE" } else { b"NUOVA VOCE" };
        font::draw(fb, w, h, x0 + 2, y0 + 2, titolo, p[0]);
        let nomi: [&[u8]; 3] = [b"SERVIZIO", b"UTENTE", b"SEGRETO"];
        font::draw(fb, w, h, x0 + 2, y0 + 12, nomi[m.campo], p[2]);
        let ing = &a.cassa.ingresso;
        if m.campo == 2 {
            let stelle = [b'*'; MAX_CAMPO];
            font::draw(fb, w, h, x0 + 2, y0 + 22, &stelle[..ing.lunghezza()], p[0]);
        } else {
            font::draw(fb, w, h, x0 + 2, y0 + 22, ing.password(), p[0]);
        }
        font::draw(fb, w, h, x0 + 2, y0 + 34, &[ing.carattere()], p[2]);
        if !a.cassa.esito.is_empty() {
            font::draw(fb, w, h, x0 + 2, y0 + 46, a.cassa.esito, p[0]);
        }
        font::draw(fb, w, h, x0 + 2, y0 + 66, b"SU GIU A L AVANTI", p[2]);
        return;
    }
    if !a.cassa.aperta {
        font::draw(fb, w, h, x0 + 2, y0 + 2, b"MASTER PASSWORD", p[0]);
        let ing = &a.cassa.ingresso;
        let stelle = [b'*'; MAX_CAMPO];
        font::draw(fb, w, h, x0 + 2, y0 + 16, &stelle[..ing.lunghezza()], p[0]);
        font::draw(fb, w, h, x0 + 2, y0 + 30, &[ing.carattere()], p[2]);
        if !a.cassa.esito.is_empty() {
            font::draw(fb, w, h, x0 + 2, y0 + 44, a.cassa.esito, p[0]);
        }
        font::draw(fb, w, h, x0 + 2, y0 + 66, b"SU GIU A L SBLOCCA", p[2]);
        return;
    }
    let n = a.cassa.archivio.len();
    if n == 0 {
        font::draw(fb, w, h, x0 + 2, y0 + 2, b"NESSUNA VOCE", p[0]);
        return;
    }
    intestazione(fb, w, h, x0 + 2, y0 + 2, b"VOCE", a.cassa.archivio.selezionata() + 1, n, p[0]);
    if let Some(v) = a.cassa.archivio.voce_selezionata() {
        font::draw(fb, w, h, x0 + 2, y0 + 12, v.servizio(), p[2]);
        font::draw(fb, w, h, x0 + 2, y0 + 22, b"UTENTE", p[0]);
        font::draw(fb, w, h, x0 + 2, y0 + 28, v.utente(), p[0]);
        font::draw(fb, w, h, x0 + 2, y0 + 38, b"SEGRETO", p[0]);
        if a.mostra {
            font::draw(fb, w, h, x0 + 2, y0 + 44, v.segreto(), p[0]);
        } else {
            let stelle = [b'*'; MAX_CAMPO];
            font::draw(fb, w, h, x0 + 2, y0 + 44, &stelle[..v.segreto().len()], p[0]);
        }
    }
    if !a.cassa.esito.is_empty() {
        font::draw(fb, w, h, x0 + 2, y0 + 56, a.cassa.esito, p[0]);
    }
    font::draw(fb, w, h, x0 + 2, y0 + 66, b"A MOSTRA R NUOVA SEL MOD", p[2]);
}

/// Dizionario: una parola alla volta, con la definizione a capo.
pub fn dizionario(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    if let Some(v) = a.dict.voce() {
        intestazione(fb, w, h, x0 + 2, y0 + 2, b"PAROLA", a.dict.posizione() + 1, a.dict.conteggio(), p[0]);
        font::draw(fb, w, h, x0 + 2, y0 + 12, v.parola.as_bytes(), p[2]);
        testo_a_capo(fb, w, h, x0 + 2, y0 + 22, v.definizione.as_bytes(), 24, 8, p[0]);
    }
}

/// Calendario: mese e anno, griglia con i giorni che hanno un evento in evidenza, primo evento del mese.
pub fn calendario(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let m = a.mese;
    let anno = m.anno as usize;
    let mm = [digit(m.mese as usize / 10), digit(m.mese as usize)];
    let s = [mm[0], mm[1], b'/', digit(anno / 1000), digit(anno / 100), digit(anno / 10), digit(anno)];
    font::draw(fb, w, h, x0 + 2, y0 + 2, &s, p[0]);
    for (k, &c) in b"LMMGVSD".iter().enumerate() {
        font::draw(fb, w, h, x0 + 2 + k * 14, y0 + 12, &[c], p[0]);
    }
    let g = griglia(m.anno, m.mese);
    for (r, riga) in g.iter().enumerate() {
        for (c, &giorno) in riga.iter().enumerate() {
            let d = giorno as usize;
            if d == 0 {
                continue;
            }
            let evento = a.cal.eventi().iter().any(|e| e.data.anno == m.anno && e.data.mese == m.mese && e.data.giorno == giorno);
            let colore = if evento { p[2] } else { p[0] };
            let cifre = [digit(d / 10), digit(d)];
            let cifre = if d < 10 { &cifre[1..] } else { &cifre[..] };
            font::draw(fb, w, h, x0 + 2 + c * 14, y0 + 20 + r * 7, cifre, colore);
        }
    }
    match a.cal.eventi().iter().find(|e| e.data.anno == m.anno && e.data.mese == m.mese) {
        Some(e) => font::draw(fb, w, h, x0 + 2, y0 + 66, e.titolo(), p[2]),
        None => font::draw(fb, w, h, x0 + 2, y0 + 66, b"NESSUN EVENTO", p[0]),
    }
}

/// Foto: elenco della pagina corrente, la selezionata con *.
pub fn foto(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let n = a.foto.count();
    if n == 0 {
        font::draw(fb, w, h, x0 + 2, y0 + 2, b"NESSUNA FOTO", p[0]);
        return;
    }
    intestazione(fb, w, h, x0 + 2, y0 + 2, b"FOTO", a.foto.selezionato() + 1, n, p[0]);
    let sel = a.foto.selezionato();
    let (inizio, fine) = a.foto.intervallo_pagina();
    for i in inizio..fine {
        let y = y0 + 12 + (i - inizio) * 10;
        if let Some(f) = a.foto.get(i) {
            if i == sel {
                font::draw(fb, w, h, x0 + 2, y, b"*", p[2]);
            }
            font::draw(fb, w, h, x0 + 8, y, f.nome(), p[0]);
        }
    }
}

/// Visualizzatore foto: la parte visibile della foto a zoom corrente, come riquadro.
pub fn visualizzatore(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let z = a.vista.zoom() as usize;
    let r = a.vista.rettangolo();
    let tw = (r.w as usize * z).min(BODY_W - 8);
    let th = (r.h as usize * z).min(BODY_H - 16);
    let ox = x0 + (BODY_W - tw) / 2;
    let oy = y0 + 10 + (BODY_H - 10 - th) / 2;
    fill(fb, w, h, ox, oy, tw, th, p[0]);
    fill(fb, w, h, ox + 1, oy + 1, tw.saturating_sub(2), th.saturating_sub(2), p[2]);
    font::draw(fb, w, h, x0 + 2, y0 + 2, &[b'Z', b'O', b'O', b'M', b' ', digit(z)], p[0]);
}

/// Mappa: griglia 32×32 a 2 px per cella, ostacoli in testo, cursore in evidenza.
pub fn mappa(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let (cx, cy) = a.cursore.pos();
    font::draw(fb, w, h, x0 + 2, y0 + 2, &[b'X', b' ', digit(cx / 10), digit(cx), b' ', b'Y', b' ', digit(cy / 10), digit(cy)], p[0]);
    let (ox, oy) = (x0 + 21, y0 + 10);
    for y in 0..LATO {
        for x in 0..LATO {
            if a.mappa.ostacolo(x, y) {
                fill(fb, w, h, ox + 2 * x, oy + 2 * y, 2, 2, p[0]);
            }
        }
    }
    fill(fb, w, h, ox + 2 * cx, oy + 2 * cy, 2, 2, p[2]);
}

/// App solo hardware: niente prova sul GBA, solo il messaggio e il componente che serve.
pub fn da_verificare(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, motivo: &[u8], p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    font::draw(fb, w, h, x0 + 2, y0 + 24, b"DA VERIFICARE", p[0]);
    font::draw(fb, w, h, x0 + 2, y0 + 34, motivo, p[2]);
}

/// Timer: round, fase e secondi restanti (tre cifre).
pub fn timer(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let t = &a.timer;
    let r = t.round() as usize;
    font::draw(fb, w, h, x0 + 2, y0 + 2, &[b'R', b'O', b'U', b'N', b'D', b' ', digit(r)], p[0]);
    let fase: &[u8] = match t.fase() {
        Fase::Lavoro => b"LAVORO",
        Fase::Riposo => b"RIPOSO",
        Fase::Fine => b"FINE",
    };
    font::draw(fb, w, h, x0 + 2, y0 + 14, fase, p[2]);
    let s = t.secondi() as usize;
    font::draw(fb, w, h, x0 + 43, y0 + 30, &[digit(s / 100), digit(s / 10), digit(s)], p[0]);
    if t.in_pausa() {
        font::draw(fb, w, h, x0 + 2, y0 + 60, b"PAUSA", p[2]);
    }
}

/// Orologio per scacchi: tempo dei due giocatori, mm:ss.
pub fn scacchi_orologio(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    for (i, riga) in [(0u8, b"BIANCO" as &[u8]), (1u8, b"NERO")] {
        let y = y0 + 12 + i as usize * 28;
        font::draw(fb, w, h, x0 + 2, y, riga, p[0]);
        let s = a.scacchi.secondi(i) as usize;
        let mmss = [digit(s / 600), digit(s / 60), b':', digit(s % 60 / 10), digit(s % 10)];
        font::draw(fb, w, h, x0 + 2, y + 12, &mmss, p[2]);
    }
}

/// Contatore: numero grande.
pub fn contatore(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    font::draw(fb, w, h, x0 + 43, y0 + 30, &a.contatore.testo(), p[0]);
}

/// Testo segnaposto: un paragrafo generato dal seme, a capo automatico.
pub fn testo_segnaposto(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let mut buf = [0u8; 96];
    let n = testo_segnaposto::genera(a.seme, &mut buf);
    testo_a_capo(fb, w, h, x0 + 2, y0 + 2, &buf[..n], 24, 8, p[0]);
}

/// Distruggi file: stato del buffer di prova e passate eseguite.
pub fn distruggi(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, a: &AppsUi, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    font::draw(fb, w, h, x0 + 2, y0 + 2, b"A: DISTRUGGI", p[0]);
    font::draw(fb, w, h, x0 + 2, y0 + 14, b"PASSATE", p[0]);
    font::draw(fb, w, h, x0 + 50, y0 + 14, &[digit(a.passate as usize)], p[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::settings::PALETTES;

    const W: usize = 240;
    const H: usize = 160;

    #[test]
    fn impostazioni_mostra_la_palette_del_tema() {
        let mut s = Settings::new();
        s.next_theme();
        let mut fb = vec![0u16; W * H];
        settings(&mut fb, W, H, 10, 10, &s, s.palette());
        for i in 0..4 {
            assert_eq!(fb[(10 + 18 + 2) * W + 10 + 4 + i * 24 + 2], PALETTES[1][i]);
        }
    }

    #[test]
    fn nota_evidenzia_il_carattere_in_attesa() {
        let p = [1u16, 31, 0x7C1F, 3];
        let mut n = Notes::new();
        n.nuova();
        let mut fb = vec![0u16; W * H];
        notes(&mut fb, W, H, 10, 10, &n, p);
        let nel_riquadro = |fb: &[u16]| {
            (0..5).any(|dy| (0..3).any(|dx| fb[(10 + TOP + dy) * W + 12 + dx] == p[2]))
        };
        assert!(!nel_riquadro(&fb)); // spazio in attesa: niente da evidenziare
        n.up(); // 'A'
        let mut fb = vec![0u16; W * H];
        notes(&mut fb, W, H, 10, 10, &n, p);
        assert!(nel_riquadro(&fb));
    }

    #[test]
    fn orologio_disegna_il_testo() {
        let p = [1u16, 31, 0x7C1F, 3];
        let mut fb = vec![0u16; W * H];
        clock(&mut fb, W, H, 10, 10, &Clock::new(9, 5), p);
        assert!(fb.iter().any(|&px| px == p[0]));
        assert_eq!(fb[(10 + 76 - 2) * W + 10 + 100], p[1]); // corpo
    }
}
