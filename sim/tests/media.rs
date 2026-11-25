//! Test host delle app media: scacchi, lettore di testo, lettore audio.

use haiku_alloc_sim::audio::Mixer;
use haiku_alloc_sim::chess::{name, sq, Board, Color, Kind, Move, Outcome, Piece};
use haiku_alloc_sim::ebook::{wrap, Reader};
use haiku_alloc_sim::player::Player;

fn mv(a: &str, b: &str) -> Move {
    Move { from: sq(a).unwrap(), to: sq(b).unwrap() }
}

fn perft(b: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    b.legal_moves()
        .into_iter()
        .map(|m| {
            let mut n = b.clone();
            assert!(n.play(m));
            perft(&n, depth - 1)
        })
        .sum()
}

// ---- Scacchi ----

#[test]
fn scacchi_perft_posizione_iniziale() {
    let b = Board::start();
    assert_eq!(perft(&b, 2), 400);
    assert_eq!(perft(&b, 3), 8902);
    assert_eq!(perft(&b, 4), 197_281);
}

#[test]
fn scacchi_nomi_di_casa() {
    assert_eq!(sq("a1"), Some(0));
    assert_eq!(sq("h8"), Some(63));
    assert_eq!(sq("i1"), None);
    assert_eq!(name(sq("e4").unwrap()), "e4");
}

#[test]
fn scacchi_mossa_illegale_rifiutata_senza_cambiare_il_tavolo() {
    let mut b = Board::start();
    let prima = b.clone();
    assert!(!b.play(mv("e2", "e5")));
    assert_eq!(b, prima);
    assert!(b.play(mv("e2", "e4")));
    assert_eq!(b.turn, Color::Black);
}

#[test]
fn scacchi_matto_del_pastore_nei_due_tratti() {
    let mut b = Board::start();
    for (da, a) in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
        assert!(b.play(mv(da, a)));
    }
    assert_eq!(b.outcome(), Outcome::Checkmate(Color::Black));
}

#[test]
fn scacchi_pezzo_inchiodato_non_si_muove() {
    // Re bianco e1, alfiere bianco e2, torre nera e8: l'alfiere è inchiodato.
    let mut b = Board::empty();
    b.put(sq("e1").unwrap(), Piece { color: Color::White, kind: Kind::King });
    b.put(sq("e2").unwrap(), Piece { color: Color::White, kind: Kind::Bishop });
    b.put(sq("e8").unwrap(), Piece { color: Color::Black, kind: Kind::Rook });
    b.put(sq("h8").unwrap(), Piece { color: Color::Black, kind: Kind::King });
    assert!(!b.legal_moves().contains(&mv("e2", "d3")));
    assert!(b.legal_moves().contains(&mv("e1", "d1")));
}

#[test]
fn scacchi_stallo() {
    // Re nero a8, donna bianca c7, re bianco b6: nero senza mosse e non sotto scacco.
    let mut b = Board::empty();
    b.put(sq("a8").unwrap(), Piece { color: Color::Black, kind: Kind::King });
    b.put(sq("c7").unwrap(), Piece { color: Color::White, kind: Kind::Queen });
    b.put(sq("b6").unwrap(), Piece { color: Color::White, kind: Kind::King });
    b.turn = Color::Black;
    assert_eq!(b.outcome(), Outcome::Stalemate);
}

#[test]
fn scacchi_promozione_automatica_a_donna() {
    let mut b = Board::empty();
    b.put(sq("b7").unwrap(), Piece { color: Color::White, kind: Kind::Pawn });
    b.put(sq("e1").unwrap(), Piece { color: Color::White, kind: Kind::King });
    b.put(sq("h8").unwrap(), Piece { color: Color::Black, kind: Kind::King });
    assert!(b.play(mv("b7", "b8")));
    assert_eq!(b.at(sq("b8").unwrap()), Some(Piece { color: Color::White, kind: Kind::Queen }));
}

// ---- Lettore di testo ----

#[test]
fn testo_a_capo_per_parola() {
    assert_eq!(wrap("ciao mondo di prova", 10), vec!["ciao mondo", "di prova"]);
}

#[test]
fn testo_parola_lunga_spezzata_e_accenti_contati_come_un_carattere() {
    assert_eq!(wrap("abcdefghijkl", 5), vec!["abcde", "fghij", "kl"]);
    assert_eq!(wrap("àèìòù", 5), vec!["àèìòù"]);
}

#[test]
fn testo_righe_vuote_restano() {
    assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
}

#[test]
fn testo_pagine_e_limiti_di_navigazione() {
    let mut r = Reader::new("1\n2\n3\n4\n5", 10, 2);
    assert_eq!(r.page_count(), 3);
    assert_eq!(r.lines(), ["1", "2"]);
    assert!(!r.prev());
    assert!(r.next() && r.next());
    assert_eq!(r.lines(), ["5"]);
    assert!(!r.next());
    assert_eq!(r.page(), 2);
}

#[test]
fn testo_vuoto_una_pagina_vuota() {
    let r = Reader::new("", 10, 5);
    assert_eq!(r.page_count(), 1);
    assert!(r.lines().is_empty());
}

// ---- Lettore audio ----

#[test]
fn audio_il_mixer_riproduce_il_brano_intero() {
    let brano: Vec<i16> = (0..300).map(|i| (i * 100 - 15_000) as i16).collect();
    let mut mixer = Mixer::new(1, 64).unwrap();
    let mut p = Player::new(0, brano.clone());
    p.play();
    let mut uscita = Vec::new();
    while !p.is_finished() || mixer.free(0).unwrap() < 64 {
        p.feed(&mut mixer).unwrap();
        uscita.extend(mixer.mix(16));
    }
    assert_eq!(&uscita[..brano.len()], &brano[..]);
}

#[test]
fn audio_in_pausa_non_accoda_nulla() {
    let mut mixer = Mixer::new(1, 64).unwrap();
    let mut p = Player::new(0, vec![7; 100]);
    assert_eq!(p.feed(&mut mixer).unwrap(), 0);
    p.play();
    assert_eq!(p.feed(&mut mixer).unwrap(), 64);
    p.pause();
    assert_eq!(p.feed(&mut mixer).unwrap(), 0);
    assert_eq!(p.position(), 64);
}

#[test]
fn audio_canale_inesistente_negato() {
    let mut mixer = Mixer::new(1, 64).unwrap();
    let mut p = Player::new(3, vec![1; 10]);
    p.play();
    assert!(p.feed(&mut mixer).is_err());
}
