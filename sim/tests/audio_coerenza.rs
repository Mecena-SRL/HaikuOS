use haiku_alloc_sim::audio::*;
use haiku_alloc_sim::cpu::CPU_MANIFEST;
use haiku_alloc_sim::crypto::CRYPTO_MANIFEST;
use haiku_alloc_sim::gpu::{GPU_EXTERNAL, GPU_GENERIC};
use haiku_alloc_sim::input::INPUT_MANIFEST;
use haiku_alloc_sim::memory::MEMORY_MANIFEST;
use haiku_alloc_sim::net::NET_MANIFEST;
use haiku_alloc_sim::power::POWER_MANIFEST;
use haiku_alloc_sim::resource::{Arch, Manifest};
use haiku_alloc_sim::storage::STORAGE_MANIFEST;
use haiku_alloc_sim::swap::SWAP_MANIFEST;
use std::collections::HashSet;

#[test]
fn mix_somma_con_volume() {
    let mut m = Mixer::new(2).unwrap();
    m.set_volume(1, 128).unwrap();
    m.push(0, &[100, 100]).unwrap();
    m.push(1, &[100, 100]).unwrap();
    assert_eq!(m.mix(2), vec![150, 150]); // 100 + 50
    assert_eq!(m.underruns(), 0);
}

#[test]
fn mix_satura_invece_di_traboccare() {
    let mut m = Mixer::new(2).unwrap();
    m.push(0, &[30000]).unwrap();
    m.push(1, &[30000]).unwrap();
    assert_eq!(m.mix(1), vec![i16::MAX]);
}

#[test]
fn canale_vuoto_e_underrun_e_silenzio() {
    let mut m = Mixer::new(1).unwrap();
    m.push(0, &[5]).unwrap();
    assert_eq!(m.mix(3), vec![5, 0, 0]);
    assert_eq!(m.underruns(), 1);
}

#[test]
fn limiti_canali() {
    assert!(Mixer::new(0).is_err());
    assert!(Mixer::new(MAX_CHANNELS + 1).is_err());
    let mut m = Mixer::new(1).unwrap();
    assert_eq!(m.push(3, &[1]), Err(AudioError::NoChannel));
}

/// Coerenza: ogni microkernel dichiara un manifest valido e i nomi sono unici.
#[test]
fn tutti_i_manifest_sono_coerenti() {
    let all: Vec<Manifest> = vec![
        CPU_MANIFEST,
        MEMORY_MANIFEST,
        STORAGE_MANIFEST,
        NET_MANIFEST,
        GPU_GENERIC,
        GPU_EXTERNAL,
        INPUT_MANIFEST,
        POWER_MANIFEST,
        CRYPTO_MANIFEST,
        SWAP_MANIFEST,
        AUDIO_MANIFEST,
    ];
    let mut names = HashSet::new();
    for m in &all {
        assert!(names.insert(m.name), "nome duplicato: {}", m.name);
        assert!(!m.archs.is_empty(), "{} senza architetture", m.name);
        assert!(m.bytes > 0, "{} senza memoria dichiarata", m.name);
        assert!(m.queue_depth > 0, "{} senza coda", m.name);
    }
}

/// Coerenza: le risorse indispensabili girano su tutte e tre le architetture.
#[test]
fn risorse_indispensabili_su_tutte_le_architetture() {
    for m in [CPU_MANIFEST, MEMORY_MANIFEST, STORAGE_MANIFEST, CRYPTO_MANIFEST] {
        assert!(m.required, "{} dovrebbe essere richiesto", m.name);
        for a in [Arch::Gba, Arch::Arm64, Arch::X86_64] {
            assert!(m.archs.contains(&a), "{} manca di {:?}", m.name, a);
        }
    }
}
