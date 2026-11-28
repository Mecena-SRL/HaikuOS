//! CLI dell'editor (versione host). Uso:
//!   haiku-editor [file] new | show | paint X Y V | fill X0 Y0 X1 Y1 V | sprite KIND X Y

use haiku_editor::{gui::{Editor, Key}, platform::{run, Platform}, Scene, Sprite};
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

    if cmd[0] == "edit" {
        edit(scene, &path);
        return;
    }

    if cmd[0] == "render" {
        let out = cmd.get(1).cloned().unwrap_or_else(|| usage());
        render(&scene, &out);
        return;
    }

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

/// Backend di prova sull'host: tasti da stdin (w/a/s/d, spazio, u, [ ], q).
struct TermPlatform {
    buf: Vec<u32>,
    pending: Vec<Key>,
}

impl Platform for TermPlatform {
    type Px = u32;
    fn size(&self) -> (usize, usize) {
        (16 * 16, 12 * 16)
    }
    fn buffer(&mut self) -> &mut [u32] {
        &mut self.buf
    }
    fn poll_key(&mut self) -> Option<Key> {
        use std::io::BufRead;
        while self.pending.is_empty() {
            let mut line = String::new();
            if std::io::stdin().lock().read_line(&mut line).ok()? == 0 {
                return None;
            }
            self.pending = line.chars().rev().filter_map(|c| match c {
                'w' => Some(Key::Up),
                's' => Some(Key::Down),
                'a' => Some(Key::Left),
                'd' => Some(Key::Right),
                ' ' => Some(Key::Paint),
                'u' => Some(Key::Undo),
                ']' => Some(Key::NextBrush),
                '[' => Some(Key::PrevBrush),
                'x' => Some(Key::Solid),
                'q' => Some(Key::Quit),
                _ => None,
            }).collect();
        }
        self.pending.pop()
    }
    fn present(&mut self) {}
}

/// Editor interattivo da terminale, sul backend di prova.
fn edit(scene: S, path: &str) {
    let mut ed: Editor<16, 12, 32, 64> = Editor::new(scene);
    let mut term = TermPlatform { buf: vec![0; 16 * 16 * 12 * 16], pending: Vec::new() };
    run(&mut ed, &mut term, 16);
    save(&ed.scene, path);
    show(&ed.scene);
}

fn save(s: &S, path: &str) {
    let mut buf = vec![0u8; S::encoded_len()];
    let n = s.encode(&mut buf).expect("buffer");
    fs::write(path, &buf[..n]).unwrap_or_else(|e| fail(&format!("{path}: {e}")));
    println!("salvato {path}");
}

/// Esporta la vista dell'editor come PPM (tile da 16 px).
fn render(s: &S, out: &str) {
    use haiku_editor::{gui::Canvas, History};
    const TS: usize = 16;
    let ed = Editor { scene: *s, hist: History::<8>::default(), cx: 0, cy: 0, brush: 1, dirty: haiku_editor::dirty::Dirty::new() };
    let (w, h) = (16 * TS, 12 * TS);
    let mut px = vec![0u32; w * h];
    ed.draw(&mut Canvas { px: &mut px, w, h }, TS);
    let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in px {
        data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
    }
    fs::write(out, data).unwrap_or_else(|e| fail(&format!("{out}: {e}")));
    println!("render {out}");
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
