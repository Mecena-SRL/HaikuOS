//! Host: `kart cup` (coppa solo IA), `kart bench` (costo per tick e per frame).

use kart::{
    race::{run_cup, Race},
    render,
    track::TRACKS,
};
use std::time::Instant;

#[path = "../../hd_shot.rs"]
mod hd_shot;

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

/// `kart shot <ow> <oh> <traccia 0-2> <tick> <out.ppm>`: griglia logica 240x135, ingrandita con `hd_shot`.
fn shot() {
    let a: Vec<String> = std::env::args().skip(2).collect();
    let (ow, oh): (usize, usize) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let track = &TRACKS[a[2].parse::<usize>().unwrap()];
    let ticks: u32 = a[3].parse().unwrap();
    let (w, h) = (240, 135);
    let mut race = Race::new(track, 8, 42);
    while race.tick < ticks && !race.finished() {
        race.tick(None);
    }
    let mut buf = vec![0u32; w * h];
    render::draw(&race, &mut buf, w, h, 0);
    let px: Vec<[u8; 3]> = buf.iter().map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
    let mut f = std::fs::File::create(&a[4]).unwrap();
    hd_shot::write_fit(&mut f, w, h, &px, ow, oh).unwrap();
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
