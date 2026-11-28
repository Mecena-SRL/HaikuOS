//! Host: `kart cup` (coppa solo IA), `kart bench` (costo per tick e per frame).

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
        Some("shot") => shot(),
        _ => eprintln!("uso: kart cup | bench"),
    }
}

/// `kart shot <w> <h> <traccia 0-2> <tick> <out.ppm>`: rende un frame di gara (PPM).
fn shot() {
    let a: Vec<String> = std::env::args().skip(2).collect();
    let (w, h): (usize, usize) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let track = &TRACKS[a[2].parse::<usize>().unwrap()];
    let ticks: u32 = a[3].parse().unwrap();
    let mut race = Race::new(track, 8, 42);
    while race.tick < ticks && !race.finished() {
        race.tick(None);
    }
    let mut buf = vec![0u32; w * h];
    render::draw(&race, &mut buf, w, h, 0);
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in &buf {
        out.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]);
    }
    std::fs::write(&a[4], out).unwrap();
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
