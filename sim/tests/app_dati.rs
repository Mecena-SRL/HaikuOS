//! Test dei modelli dati di mail e rubrica (solo host). Vedi docs/apps-connected.md.
use haiku_alloc_sim::contacts::*;
use haiku_alloc_sim::mail::*;

#[test]
fn indirizzo_valido() {
    assert!(valid_address("a@b.it"));
    assert!(!valid_address("ab.it"));
    assert!(!valid_address("a@@b.it"));
    assert!(!valid_address("@b.it"));
    assert!(!valid_address("a@bit"));
    assert!(!valid_address("a@.it"));
}

#[test]
fn mail_aggiunta_lettura_e_ordine() {
    let mut mb = Mailbox::new();
    let a = mb.add("x@y.it", "me@h.it", "ciao", "corpo").unwrap();
    let b = mb.add("z@y.it", "me@h.it", "due", "").unwrap();
    assert_eq!(mb.unread_count(), 2);
    mb.mark_read(a).unwrap();
    assert_eq!(mb.unread_count(), 1);
    assert_eq!(mb.get(a).unwrap().body, "corpo");
    assert_eq!(mb.list()[0].id, b);
}

#[test]
fn mail_indirizzo_e_limiti() {
    let mut mb = Mailbox::new();
    assert_eq!(mb.add("male", "me@h.it", "s", "b"), Err(MailError::BadAddress));
    let big = "x".repeat(MAX_BODY_BYTES + 1);
    assert_eq!(mb.add("x@y.it", "me@h.it", "s", &big), Err(MailError::TooLarge));
    assert!(mb.is_empty());
}

#[test]
fn mail_casella_piena_e_rimozione() {
    let mut mb = Mailbox::new();
    for _ in 0..MAX_MESSAGES {
        mb.add("x@y.it", "me@h.it", "s", "b").unwrap();
    }
    assert_eq!(mb.add("x@y.it", "me@h.it", "s", "b"), Err(MailError::Full));
    let id = mb.list()[0].id;
    assert!(mb.remove(id).is_ok());
    assert_eq!(mb.remove(id), Err(MailError::NotFound));
    assert_eq!(mb.mark_read(9999), Err(MailError::NotFound));
    assert!(mb.add("x@y.it", "me@h.it", "s", "b").is_ok());
}

#[test]
fn rubrica_aggiunta_ricerca_e_ordine() {
    let mut bk = Book::new();
    bk.add("Marco", &["+39021234567"], &["marco@x.it"]).unwrap();
    bk.add("maria", &[], &[]).unwrap();
    bk.add("Luca", &["3331112222"], &[]).unwrap();
    let r: Vec<&str> = bk.search("ma").iter().map(|c| c.name.as_str()).collect();
    assert_eq!(r, vec!["Marco", "maria"]);
    assert_eq!(bk.find_by_address("marco@x.it").unwrap().name, "Marco");
    assert_eq!(bk.find_by_address("3331112222").unwrap().name, "Luca");
    assert!(bk.find_by_address("nessuno@x.it").is_none());
}

#[test]
fn rubrica_validazione() {
    let mut bk = Book::new();
    assert_eq!(bk.add("  ", &[], &[]), Err(ContactError::BadName));
    assert_eq!(bk.add(&"n".repeat(MAX_NAME_BYTES + 1), &[], &[]), Err(ContactError::BadName));
    assert_eq!(bk.add("A", &["12a"], &[]), Err(ContactError::BadPhone));
    assert_eq!(bk.add("A", &["+"], &[]), Err(ContactError::BadPhone));
    assert_eq!(bk.add("A", &[], &["nope"]), Err(ContactError::BadEmail));
    assert_eq!(bk.add("A", &["111", "222", "333", "444", "555"], &[]), Err(ContactError::TooManyFields));
    assert!(bk.is_empty());
}

#[test]
fn rubrica_modifica_rimozione_e_piena() {
    let mut bk = Book::new();
    let id = bk.add("Anna", &["123"], &[]).unwrap();
    bk.update(id, "Anna B", &["456"], &["a@b.it"]).unwrap();
    assert_eq!(bk.get(id).unwrap().name, "Anna B");
    assert_eq!(bk.update(999, "X", &[], &[]), Err(ContactError::NotFound));
    assert_eq!(bk.update(id, "", &[], &[]), Err(ContactError::BadName));
    assert_eq!(bk.get(id).unwrap().name, "Anna B"); // invariato dopo errore
    assert_eq!(bk.remove(id).unwrap().name, "Anna B");
    assert_eq!(bk.remove(id), Err(ContactError::NotFound));
    for i in 0..MAX_CONTACTS {
        bk.add(&format!("c{i}"), &[], &[]).unwrap();
    }
    assert_eq!(bk.add("extra", &[], &[]), Err(ContactError::Full));
}
