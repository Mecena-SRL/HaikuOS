use fontmgr::*;
use haiku_alloc_sim::fonts::FontManager;
use haiku_alloc_sim::opentype::synth_font;

fn no_files(_: &str) -> Result<Vec<u8>, String> {
    Err("nessun file".into())
}

fn run(fm: &mut FontManager, line: &str) -> Result<String, AppError> {
    exec(fm, line, &|p: &str| if p == "a.ttf" { Ok(synth_font("Alfa")) } else { no_files(p) })
}

#[test]
fn installa_da_file_e_anteprima() {
    let mut fm = FontManager::new();
    assert_eq!(run(&mut fm, "install a.ttf").unwrap(), "installato: Alfa");
    assert_eq!(run(&mut fm, "preview Alfa Ciao mondo").unwrap(), "Ciao mondo");
    assert_eq!(run(&mut fm, "install a.ttf"), Err(AppError::Font(haiku_alloc_sim::fonts::FontError::Duplicate)));
    assert_eq!(run(&mut fm, "install x.ttf"), Err(AppError::Io("nessun file".into())));
}

#[test]
fn attiva_disattiva_e_sistema() {
    let mut fm = FontManager::new();
    run(&mut fm, "install a.ttf").unwrap();
    assert!(run(&mut fm, "list").unwrap().contains("Alfa inattivo"));
    run(&mut fm, "activate Alfa").unwrap();
    assert!(run(&mut fm, "list").unwrap().contains("Alfa attivo"));
    assert_eq!(run(&mut fm, "system Alfa").unwrap(), "font di sistema impostato");
    assert!(run(&mut fm, "deactivate Alfa").is_err());
    assert!(run(&mut fm, "list").unwrap().contains("(sistema)"));
}

#[test]
fn collezioni_e_nomi_sconosciuti() {
    let mut fm = FontManager::new();
    run(&mut fm, "install a.ttf").unwrap();
    assert_eq!(run(&mut fm, "collection Giochi").unwrap(), "collezione 0");
    assert_eq!(run(&mut fm, "add 0 Alfa").unwrap(), "aggiunto");
    assert_eq!(run(&mut fm, "add 9 Alfa"), Err(AppError::BadIndex("9".into())));
    assert_eq!(run(&mut fm, "activate Zeta"), Err(AppError::UnknownName("Zeta".into())));
    assert_eq!(run(&mut fm, "fai"), Err(AppError::UnknownCommand("fai".into())));
    assert_eq!(run(&mut fm, "activate"), Err(AppError::MissingArg("nome")));
}

#[test]
fn file_non_valido_non_installa() {
    let mut fm = FontManager::new();
    let bad = |_: &str| Ok(b"non un font".to_vec());
    assert!(matches!(exec(&mut fm, "install b.ttf", &bad), Err(AppError::Parse(_))));
    assert!(fm.find("Alfa").is_none());
}
