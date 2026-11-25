use haiku_alloc_sim::crypto::*;
use haiku_alloc_sim::input::*;
use haiku_alloc_sim::power::*;
use haiku_alloc_sim::swap::*;

#[test]
fn input_coda_piena_scarta_e_conta() {
    let mut i = Input::new(2);
    assert!(i.push(Event::Press(Button::A)).is_ok());
    assert!(i.push(Event::Release(Button::A)).is_ok());
    assert_eq!(i.push(Event::Press(Button::B)), Err(haiku_alloc_sim::resource::Error::Full));
    assert_eq!(i.dropped(), 1);
    assert_eq!(i.poll(), Some(Event::Press(Button::A)));
    assert_eq!(i.poll(), Some(Event::Release(Button::A)));
    assert_eq!(i.poll(), None);
}

#[test]
fn power_transizioni_ammesse_e_rifiutate() {
    let mut p = Power::new(80, 5);
    assert_eq!(p.request(State::Sleep), Err(PowerError::Illegal)); // Off→Sleep non ammesso
    assert!(p.request(State::Active).is_ok());
    assert!(p.request(State::Idle).is_ok());
    assert!(p.request(State::Sleep).is_ok()); // Idle→Sleep ammesso
}

#[test]
fn power_batteria_critica_blocca_il_ritorno_ad_active() {
    let mut p = Power::new(80, 5);
    p.request(State::Active).unwrap();
    p.request(State::Idle).unwrap();
    p.request(State::Sleep).unwrap();
    p.set_battery(3);
    assert_eq!(p.request(State::Active), Err(PowerError::LowBattery));
    assert_eq!(p.state(), State::Sleep);
    p.set_battery(50);
    assert!(p.request(State::Active).is_ok());
}

#[test]
fn crypto_round_trip_e_nonce_diversi() {
    let mut v = Vault::new([7; 16], 2, 512);
    let a = v.seal(0, b"ciao haiku").unwrap();
    let b = v.seal(0, b"ciao haiku").unwrap();
    assert_ne!(a, b); // stesso testo, nonce diverso
    assert_eq!(v.open(0, &a).unwrap(), b"ciao haiku");
}

#[test]
fn crypto_slot_sbagliato_e_alterazione_rifiutati() {
    let mut v = Vault::new([7; 16], 2, 512);
    let mut s = v.seal(0, b"segreto").unwrap();
    assert!(v.open(1, &s).is_err());
    let last = s.len() - 1;
    s[last] ^= 1;
    assert!(v.open(0, &s).is_err());
}

#[test]
fn swap_pagina_compressa_round_trip() {
    let mut sw = Swap::new(2, Vault::new([3; 16], 1, 512));
    let page = vec![0xAB; PAGE_BYTES];
    let blk = sw.page_out(&page).unwrap();
    assert_eq!(sw.page_in(blk).unwrap(), page);
}

#[test]
fn swap_blocco_alterato_rilevato() {
    let mut sw = Swap::new(1, Vault::new([3; 16], 1, 512));
    let blk = sw.page_out(&vec![1u8; PAGE_BYTES]).unwrap();
    sw.inject_fault(blk, 10);
    assert!(sw.page_in(blk).is_err());
}

#[test]
fn swap_esaurito_e_pagina_casuale_troppo_grande() {
    let mut sw = Swap::new(1, Vault::new([3; 16], 1, 512));
    sw.page_out(&vec![0u8; PAGE_BYTES]).unwrap();
    assert_eq!(sw.page_out(&vec![0u8; PAGE_BYTES]), Err(SwapError::NoFreeBlock));
    let random: Vec<u8> = (0..PAGE_BYTES).map(|i| (i as u32).wrapping_mul(2654435761) as u8).collect();
    let mut sw2 = Swap::new(1, Vault::new([3; 16], 1, 512));
    assert_eq!(sw2.page_out(&random), Err(SwapError::TooLarge));
}
