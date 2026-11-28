//! Host: `kart cup` (coppa solo IA), `kart bench` (costo per tick e per frame),
//! `kart shot W H out.ppm [tick]` (fotogramma 3D in PPM).

use kart::{
    race::{run_cup, Race},
    render,
    track::TRACKS,
};
use std::time::Instant;

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("cup") => {
            for (i, p) in run_cup(&TRACKS, 8).iter().enumerate() {
                println!("kart {i}: {p} punti");
            }
        }
        Some("bench") => bench(),
        Some("shot") => {
            let a: Vec<String> = std::env::args().collect();
            let num = |i: usize, d: usize| a.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
            shot(num(2, 800), num(3, 480), a.get(4).map(String::as_str).unwrap_or("shot.ppm"), num(5, 240));
        }
        _ => eprintln!("uso: kart cup | bench | shot W H out.ppm [tick]"),
    }
}

fn bench() {
    let track = &TRACKS[0];
    let mut buf = vec![0u32; 640 * 480];

    let mut race = Race::new(track, 8, 42);
    let t0 = Instant::now();
    while !race.finished() && race.tick < 60_000 {
        race.tick(None);
    }
    let ticks = race.tick;
    let sim_ns = t0.elapsed().as_nanos() as f64 / ticks as f64;

    let mut race = Race::new(track, 8, 42);
    let t1 = Instant::now();
    const FRAMES: u32 = 300;
    for _ in 0..FRAMES {
        race.tick(None);
        render::draw(&race, &mut buf, 640, 480, 0);
    }
    let frame_us = t1.elapsed().as_micros() as f64 / FRAMES as f64;

    println!("pista {}: gara finita in {ticks} tick", track.name);
    println!("simulazione: {:.2} us/tick (8 kart)", sim_ns / 1000.0);
    println!("frame sim+render 640x480: {frame_us:.1} us");
}

/// Un fotogramo dopo `ticks` tick di gara (guida l'IA anche il giocatore, kart 0).
fn shot(w: usize, h: usize, path: &str, ticks: usize) {
    let mut race = Race::new(&TRACKS[0], 4, 42);
    for _ in 0..ticks {
        race.tick(None);
    }
    let mut buf = vec![0u32; w * h];
    render::draw(&race, &mut buf, w, h, 0);
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in &buf {
        out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    std::fs::write(path, out).expect("scrittura PPM");
}
