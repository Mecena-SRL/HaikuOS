//! Esporta desktop e splash in PPM alla risoluzione richiesta.
//! Uso: cargo run -p haiku-ui --example render -- <cartella> <larghezza> <altezza>

use haiku_editor::gui::Canvas;
use haiku_ui::{desktop, splash};
use std::{env, fs};

fn write(path: String, buf: &[u32], w: usize, h: usize) {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in buf {
        out.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    fs::write(path, out).expect("scrittura fallita");
}

fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let (dir, w, h) = (&a[0], a[1].parse::<usize>().unwrap(), a[2].parse::<usize>().unwrap());
    let mut buf = vec![0u32; w * h];
    desktop(&mut Canvas { px: &mut buf, w, h });
    write(format!("{dir}/desktop_{w}x{h}.ppm"), &buf, w, h);
    splash(&mut Canvas { px: &mut buf, w, h }, 600);
    write(format!("{dir}/splash_{w}x{h}.ppm"), &buf, w, h);
}
