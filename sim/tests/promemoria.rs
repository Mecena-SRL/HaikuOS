//! Test del modello promemoria (solo host). Vedi docs/apps-connected.md.
use haiku_alloc_sim::reminders::*;

#[test]
fn aggiunta_e_scadenze_in_ordine() {
    let mut r = Reminders::new();
    let a = r.add("pagare", 100).unwrap();
    let b = r.add("chiamare", 50).unwrap();
    r.add("futuro", 500).unwrap();
    let due: Vec<u32> = r.due_at(120).iter().map(|x| x.id).collect();
    assert_eq!(due, vec![b, a]);
    assert_eq!(r.next_due(), Some(50));
}

#[test]
fn completato_non_scade_piu() {
    let mut r = Reminders::new();
    let a = r.add("x", 10).unwrap();
    r.complete(a).unwrap();
    assert!(r.due_at(100).is_empty());
    assert_eq!(r.next_due(), None);
    assert_eq!(r.complete(99), Err(ReminderError::NotFound));
}

#[test]
fn testo_non_valido() {
    let mut r = Reminders::new();
    assert_eq!(r.add("   ", 1), Err(ReminderError::BadText));
    assert_eq!(r.add(&"x".repeat(MAX_TEXT_BYTES + 1), 1), Err(ReminderError::BadText));
    assert!(r.is_empty());
}

#[test]
fn lista_piena_e_rimozione() {
    let mut r = Reminders::new();
    for i in 0..MAX_REMINDERS {
        r.add("t", i as u64).unwrap();
    }
    assert_eq!(r.add("t", 0), Err(ReminderError::Full));
    let rimosso = r.remove(1).unwrap();
    assert_eq!(rimosso.id, 1);
    assert_eq!(r.remove(1), Err(ReminderError::NotFound));
    assert!(r.add("t", 0).is_ok());
}
