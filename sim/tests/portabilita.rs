use haiku_alloc_sim::install::*;
use std::cell::RefCell;

const ALL: &[Arch] = &[Arch::Arm7tdmi, Arch::Arm64, Arch::X86_64];
const ONLY_64: &[Arch] = &[Arch::Arm64, Arch::X86_64];
const RAM: usize = 288 * 1024;

fn comp(name: &'static str, archs: &'static [Arch], required: bool) -> Component {
    Component { name, archs, required, bytes: 1024 }
}

#[test]
fn componente_incompatibile_opzionale_non_caricato() {
    let comps = [comp("nucleo", ALL, true), comp("gpu-x86", ONLY_64, false)];
    let mut loaded = Vec::new();
    let r = install(Arch::Arm7tdmi, RAM, &comps, |n| { loaded.push(n); true }, |_| {});
    assert_eq!(r, Ok(vec!["nucleo"]));
    assert_eq!(loaded, vec!["nucleo"]);
}

#[test]
fn componente_richiesto_incompatibile_non_carica_nulla() {
    let comps = [comp("nucleo", ALL, true), comp("avvio-x86", ONLY_64, true)];
    let mut calls = 0;
    let r = install(Arch::Arm7tdmi, RAM, &comps, |_| { calls += 1; true }, |_| {});
    assert_eq!(r, Err(InstallError::RequiredIncompatible("avvio-x86")));
    assert_eq!(calls, 0);
}

#[test]
fn ram_insufficiente_non_carica_nulla() {
    let comps = [Component { name: "grande", archs: ALL, required: true, bytes: RAM + 1 }];
    let mut calls = 0;
    let r = install(Arch::Arm64, RAM, &comps, |_| { calls += 1; true }, |_| {});
    assert_eq!(r, Err(InstallError::OutOfRam));
    assert_eq!(calls, 0);
}

#[test]
fn attivazione_fallita_fa_rollback_senza_stato_parziale() {
    let comps = [comp("a", ALL, true), comp("b", ALL, true), comp("c", ALL, true)];
    let active = RefCell::new(Vec::new());
    let undone = RefCell::new(Vec::new());
    let r = install(
        Arch::Arm64,
        RAM,
        &comps,
        |n| {
            if n == "c" { return false; }
            active.borrow_mut().push(n);
            true
        },
        |n| {
            undone.borrow_mut().push(n);
            active.borrow_mut().retain(|x| *x != n);
        },
    );
    assert_eq!(r, Err(InstallError::ActivationFailed("c")));
    assert!(active.borrow().is_empty(), "stato parziale: {:?}", active.borrow());
    assert_eq!(*undone.borrow(), vec!["b", "a"]);
}
