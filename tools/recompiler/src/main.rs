//! Ricompilatore v0: traduce un sorgente C in Rust `no_std` con un LLM
//! e accetta il risultato solo se i test di specifica passano.
//!
//! Uso: recompiler <sorgente.c> <test.rs> <cartella_out>
//! Env: DEEPSEEK_API_KEY (obbligatoria), RECOMP_MODEL, RECOMP_URL,
//!      RECOMP_MOCK=<file> (risposta fissa, per test offline).

use serde_json::{json, Value};
use std::{
    env, fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const SYSTEM: &str = "Traduci C in Rust. Regole: #![no_std], niente alloc, niente crate esterni, \
niente unsafe se non indispensabile. Mantieni nomi e comportamento pubblici. \
Puntatore+lunghezza diventa &[u8]. Tipi a larghezza fissa (unsigned long -> u64). \
Overflow intenzionale del C: usa wrapping_*. Rispondi solo con un blocco ```rust.";
const TRIES: usize = 3;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (src, spec, out) = match args.as_slice() {
        [a, b, c] => (a, b, c),
        _ => {
            eprintln!("uso: recompiler <sorgente.c> <test.rs> <cartella_out>");
            std::process::exit(2)
        }
    };
    let c_code = fs::read_to_string(src).expect("leggo il sorgente");
    let spec_code = fs::read_to_string(spec).expect("leggo i test");
    let out = Path::new(out);
    fs::create_dir_all(out.join("src")).unwrap();
    fs::create_dir_all(out.join("tests")).unwrap();
    fs::write(
        out.join("Cargo.toml"),
        "[package]\nname = \"translated\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(out.join("tests/spec.rs"), &spec_code).unwrap();

    let mut user = format!("Sorgente C:\n```c\n{c_code}\n```");
    for attempt in 1..=TRIES {
        let reply = match env::var("RECOMP_MOCK") {
            Ok(p) => fs::read_to_string(p).expect("leggo la risposta mock"),
            Err(_) => ask_llm(out, &user).unwrap_or_else(|e| {
                eprintln!("API: {e}");
                std::process::exit(1)
            }),
        };
        let code = extract_rust(&reply);
        fs::write(out.join("src/lib.rs"), &code).unwrap();
        match run_tests(out) {
            Ok(()) => {
                println!("OK al tentativo {attempt}: {}", out.join("src/lib.rs").display());
                return;
            }
            Err(log) => {
                eprintln!("tentativo {attempt} fallito");
                user = format!(
                    "{user}\n\nRust precedente:\n```rust\n{code}\n```\nErrori:\n{log}\n\
                     Correggi e rispondi solo con il blocco rust completo."
                );
            }
        }
    }
    std::process::exit(1);
}

/// Chiama un endpoint compatibile OpenAI (DeepSeek di default).
/// La chiave passa via config di curl su stdin, mai in argv.
fn ask_llm(out: &Path, user: &str) -> Result<String, String> {
    let key = env::var("DEEPSEEK_API_KEY").map_err(|_| "manca DEEPSEEK_API_KEY")?;
    let url = env::var("RECOMP_URL").unwrap_or("https://api.deepseek.com/chat/completions".into());
    let model = env::var("RECOMP_MODEL").unwrap_or("deepseek-chat".into());
    let body = json!({
        "model": model,
        "temperature": 0,
        "messages": [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}]
    });
    let body_path = out.join(".body.json");
    fs::write(&body_path, body.to_string()).map_err(|e| e.to_string())?;
    let config = format!(
        "url = \"{url}\"\nheader = \"Authorization: Bearer {key}\"\n\
         header = \"Content-Type: application/json\"\ndata-binary = \"@{}\"\n",
        body_path.display()
    );
    let mut child = Command::new("curl")
        .args(["-sS", "-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child.stdin.take().unwrap().write_all(config.as_bytes()).map_err(|e| e.to_string())?;
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&body_path);
    let resp = String::from_utf8_lossy(&o.stdout).to_string();
    let v: Value = serde_json::from_str(&resp).map_err(|e| format!("{e}: {resp}"))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("risposta inattesa: {resp}"))
}

/// Prende il primo blocco ```...``` della risposta, o tutto il testo se non c'è.
fn extract_rust(reply: &str) -> String {
    match reply.find("```") {
        Some(i) => {
            let s = &reply[i + 3..];
            let body = s.find('\n').map_or(s, |n| &s[n + 1..]);
            format!("{}\n", body.split("```").next().unwrap_or(body).trim_end())
        }
        None => reply.to_string(),
    }
}

/// `cargo test` nella cartella generata. Ok solo se compila e i test passano.
fn run_tests(dir: &Path) -> Result<(), String> {
    let o = Command::new("cargo")
        .args(["test", "-q"])
        .current_dir(dir)
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        return Ok(());
    }
    let log = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    let tail: Vec<char> = log.chars().collect();
    Err(tail[tail.len().saturating_sub(3000)..].iter().collect())
}
