//! Test host: `cd gba && rustc --edition 2021 --test host/test_calc.rs -o /tmp/calc_t && /tmp/calc_t`
#![allow(dead_code)]
#[path = "../src/calc.rs"]
mod calc;

use calc::{apply, format, Calc, Err, Op};

fn fmt(v: i32) -> String {
    let mut b = [0u8; 12];
    let n = format(v, &mut b);
    String::from_utf8(b[..n].to_vec()).unwrap()
}

/// Digita un numero come stringa, es. "12.5".
fn type_num(c: &mut Calc, s: &str) {
    for ch in s.chars() {
        match ch {
            '.' => c.point(),
            d => c.digit(d as u8 - b'0'),
        }
    }
}

#[test]
fn quattro_operazioni() {
    assert_eq!(apply(150, Op::Add, 275), Ok(425)); // 1,50 + 2,75
    assert_eq!(apply(500, Op::Sub, 125), Ok(375));
    assert_eq!(apply(150, Op::Mul, 200), Ok(300)); // 1,50 * 2
    assert_eq!(apply(300, Op::Div, 200), Ok(150)); // 3 / 2
    assert_eq!(apply(100, Op::Div, 300), Ok(33)); // 1 / 3 = 0,33 (troncato)
}

#[test]
fn divisione_per_zero() {
    assert_eq!(apply(100, Op::Div, 0), Err(Err::DivZero));
}

#[test]
fn overflow() {
    assert_eq!(apply(i32::MAX, Op::Add, 1), Err(Err::Overflow));
    assert_eq!(apply(i32::MIN, Op::Sub, 1), Err(Err::Overflow));
}

#[test]
fn formato_decimali() {
    assert_eq!(fmt(0), "0.00");
    assert_eq!(fmt(5), "0.05");
    assert_eq!(fmt(1250), "12.50");
    assert_eq!(fmt(-1205), "-12.05");
    assert_eq!(fmt(i32::MAX), "21474836.47");
    assert_eq!(fmt(i32::MIN), "-21474836.48");
}

#[test]
fn somma_da_tastiera() {
    let mut c = Calc::new();
    type_num(&mut c, "1.5");
    c.op(Op::Add);
    type_num(&mut c, "2.75");
    c.equals();
    assert_eq!(c.shown(), 425);
    assert_eq!(fmt(c.shown()), "4.25");
}

#[test]
fn catena_di_operazioni() {
    let mut c = Calc::new();
    type_num(&mut c, "2");
    c.op(Op::Mul);
    type_num(&mut c, "3");
    c.op(Op::Sub); // calcola 2*3 = 6, poi attende un operatore
    assert_eq!(c.shown(), 600);
    type_num(&mut c, "0.5");
    c.equals();
    assert_eq!(c.shown(), 550);
}

#[test]
fn due_cifre_dopo_la_virgola() {
    let mut c = Calc::new();
    type_num(&mut c, "1.234");
    assert_eq!(c.shown(), 123); // la terza cifra si ignora
}

#[test]
fn virgola_una_volta_sola_e_zero_iniziale() {
    let mut c = Calc::new();
    c.point();
    c.point();
    c.digit(7);
    assert_eq!(c.shown(), 70); // prima cifra dopo la virgola = decimi
    assert_eq!(fmt(c.shown()), "0.70");
}

#[test]
fn divisione_per_zero_blocca_fino_a_clear() {
    let mut c = Calc::new();
    type_num(&mut c, "1");
    c.op(Op::Div);
    c.digit(0);
    c.equals();
    assert_eq!(c.err, Some(Err::DivZero));
    c.digit(5); // ignorato
    assert_eq!(c.shown(), 0);
    c.clear();
    assert_eq!(c.err, None);
    type_num(&mut c, "4");
    assert_eq!(c.shown(), 400);
}

#[test]
fn cifra_dopo_uguale_ricomincia() {
    let mut c = Calc::new();
    type_num(&mut c, "2");
    c.op(Op::Add);
    type_num(&mut c, "2");
    c.equals();
    assert_eq!(c.shown(), 400);
    type_num(&mut c, "1");
    assert_eq!(c.shown(), 100);
}

#[test]
fn overflow_di_inserimento_ignorato() {
    let mut c = Calc::new();
    for _ in 0..12 {
        c.digit(9);
    }
    assert!(c.shown() > 0 && c.shown() < i32::MAX);
}
