use fontmgr::*;
use haiku_alloc_sim::fonts::FontError;
use haiku_alloc_sim::opentype::synth_font;

fn run(s: &mut Session, line: &str) -> Result<String, AppError> {
    exec(s, line, &|p: &str| if p == "a.ttf" { Ok(synth_font("Alfa")) } else { Err("nessun file".into()) })
}

#[test]
fn installa_da_file() {
    let mut s = Session::new();
    assert_eq!(run(&mut s, "install a.ttf").unwrap(), "installato: Alfa");
    assert_eq!(run(&mut s, "install a.ttf"), Err(AppError::Font(FontError::Duplicate)));
    assert_eq!(run(&mut s, "install x.ttf"), Err(AppError::Io("nessun file".into())));
}

#[test]
fn anteprima_disegna_il_glifo() {
    let mut s = Session::new();
    run(&mut s, "install a.ttf").unwrap();
    // 16 px/em: altezza 16 righe; 'A' largo 600 unità = 10 px.
    let art = run(&mut s, "preview Alfa A").unwrap();
    let righe: Vec<&str> = art.lines().collect();
    assert_eq!(righe.len(), 16);
    assert!(righe.iter().any(|r| r.contains('#')));
    assert!(righe[0].trim().is_empty());
    // Senza glifo ('Z'): anteprima vuota.
    let vuota = run(&mut s, "preview Alfa Z").unwrap();
    assert!(vuota.chars().all(|c| c == ' ' || c == '\n'));
}

#[test]
fn attiva_disattiva_e_sistema() {
    let mut s = Session::new();
    run(&mut s, "install a.ttf").unwrap();
    assert!(run(&mut s, "list").unwrap().contains("Alfa inattivo"));
    run(&mut s, "activate Alfa").unwrap();
    assert!(run(&mut s, "list").unwrap().contains("Alfa attivo"));
    assert_eq!(run(&mut s, "system Alfa").unwrap(), "font di sistema impostato");
    assert!(run(&mut s, "deactivate Alfa").is_err());
    assert!(run(&mut s, "list").unwrap().contains("(sistema)"));
}

#[test]
fn collezioni_e_errori_di_comando() {
    let mut s = Session::new();
    run(&mut s, "install a.ttf").unwrap();
    assert_eq!(run(&mut s, "collection Giochi").unwrap(), "collezione 0");
    assert_eq!(run(&mut s, "add 0 Alfa").unwrap(), "aggiunto");
    assert_eq!(run(&mut s, "add 9 Alfa"), Err(AppError::BadIndex("9".into())));
    assert_eq!(run(&mut s, "activate Zeta"), Err(AppError::UnknownName("Zeta".into())));
    assert_eq!(run(&mut s, "fai"), Err(AppError::UnknownCommand("fai".into())));
    assert_eq!(run(&mut s, "activate"), Err(AppError::MissingArg("nome")));
}

#[test]
fn file_non_valido_non_installa() {
    let mut s = Session::new();
    let bad = |_: &str| Ok(b"non un font".to_vec());
    assert!(matches!(exec(&mut s, "install b.ttf", &bad), Err(AppError::Parse(_))));
    assert!(s.fm.find("Alfa").is_none());
}
