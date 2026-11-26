//! CLI dell'editor (versione host). Uso:
//!   haiku-editor [file] new | show | paint X Y V | fill X0 Y0 X1 Y1 V | sprite KIND X Y

use haiku_editor::{Scene, Sprite};
use std::{env, fs, process};

type S = Scene<16, 12, 32>;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (path, cmd) = match args.as_slice() {
        [p, rest @ ..] if !rest.is_empty() => (p.clone(), rest.to_vec()),
        _ => usage(),
    };
    let num = |i: usize| -> usize { cmd.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| usage()) };

    let mut scene = if cmd[0] == "new" {
        S::default()
    } else {
        let buf = fs::read(&path).unwrap_or_else(|e| fail(&format!("{path}: {e}")));
        S::decode(&buf).unwrap_or_else(|| fail("file scena non valido"))
    };

    match cmd[0].as_str() {
        "new" => {}
        "paint" => {
            let (x, y) = (num(1), num(2));
            scene.tiles.set(x, y, num(3) as u8).unwrap_or_else(|| fail("fuori griglia"));
        }
        "fill" => scene.tiles.fill_rect(num(1), num(2), num(3), num(4), num(5) as u8),
        "sprite" => {
            let s = Sprite { kind: num(1) as u8, x: num(2) as u16, y: num(3) as u16 };
            scene.add_sprite(s).unwrap_or_else(|| fail("sprite pieni"));
        }
        "show" => {}
        _ => usage(),
    }

    if cmd[0] == "show" {
        show(&scene);
    } else {
        let mut buf = vec![0u8; S::encoded_len()];
        let n = scene.encode(&mut buf).expect("buffer");
        fs::write(&path, &buf[..n]).unwrap_or_else(|e| fail(&format!("{path}: {e}")));
        println!("salvato {path}");
    }
}

fn show(s: &S) {
    for row in s.tiles.cells.iter() {
        let line: String = row.iter().map(|&v| if v == 0 { '.' } else { char::from(b'0' + v % 10) }).collect();
        println!("{line}");
    }
    for (i, slot) in s.sprites.iter().enumerate() {
        if let Some(sp) = slot {
            println!("sprite {i}: kind={} x={} y={}", sp.kind, sp.x, sp.y);
        }
    }
}

fn usage() -> ! {
    eprintln!("uso: haiku-editor <file> new | show | paint X Y V | fill X0 Y0 X1 Y1 V | sprite KIND X Y");
    process::exit(2)
}

fn fail(msg: &str) -> ! {
    eprintln!("errore: {msg}");
    process::exit(1)
}
