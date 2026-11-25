use haiku_alloc_sim::audio::{Mixer, AUDIO_MANIFEST};
use haiku_alloc_sim::crypto::{Vault, CRYPTO_MANIFEST};
use haiku_alloc_sim::gpu::{Cmd, Gpu, GPU_GENERIC};
use haiku_alloc_sim::input::{Button, Event, Input, INPUT_MANIFEST};
use haiku_alloc_sim::memory::{MemoryKernel, MEMORY_MANIFEST};
use haiku_alloc_sim::power::{Power, POWER_MANIFEST};
use haiku_alloc_sim::resource::{Arch, Driver, Error, Manifest, Microkernel, Queue, Request, Completion, tick};
use haiku_alloc_sim::storage::{Device, STORAGE_MANIFEST};
use haiku_alloc_sim::swap::{Swap, SWAP_MANIFEST};

const ALL: &[Arch] = &[Arch::Gba, Arch::Arm64, Arch::X86_64];

fn m(name: &'static str) -> Manifest {
    Manifest { name, kind: haiku_alloc_sim::resource::Kind::Gpu, archs: ALL, required: true, bytes: 1, queue_depth: 1 }
}

#[test]
fn coda_fissa_piena_e_ordinata() {
    let mut q: Queue<u8> = Queue::new(2);
    q.push(1).unwrap();
    q.push(2).unwrap();
    assert_eq!(q.push(3), Err(Error::Full));
    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.drain_all(), vec![2]);
    assert!(q.is_empty());
}

#[test]
fn ogni_modulo_dichiara_il_proprio_manifest() {
    let mems: Vec<Box<dyn Microkernel>> = vec![
        Box::new(Mixer::new(1, 8).unwrap()),
        Box::new(Power::new(50, 5)),
        Box::new(Vault::new([0; 16], 1, 64)),
        Box::new(Swap::new(1, Vault::new([0; 16], 1, 64))),
        Box::new(Device::new(1, 4)),
        Box::new(MemoryKernel::new(1, 1, 4, 1)),
        Box::new(Gpu::new(4, 1)),
        Box::new(Input::new(1)),
        Box::new(Driver::new(m("x"))),
    ];
    let names: Vec<&str> = mems.iter().map(|k| k.manifest().name).collect();
    assert_eq!(names.len(), 9);
    assert_eq!(mems[0].manifest(), AUDIO_MANIFEST);
    assert_eq!(mems[1].manifest(), POWER_MANIFEST);
    assert_eq!(mems[2].manifest(), CRYPTO_MANIFEST);
    assert_eq!(mems[4].manifest(), STORAGE_MANIFEST);
    assert_eq!(mems[5].manifest(), MEMORY_MANIFEST);
    assert_eq!(mems[6].manifest(), GPU_GENERIC);
    assert_eq!(mems[7].manifest(), INPUT_MANIFEST);
    assert_eq!(mems[3].manifest(), SWAP_MANIFEST);
    assert_eq!(names[8], "x");
}

#[test]
fn gpu_e_input_usano_la_coda_generica() {
    let mut g = Gpu::new(4, 1);
    g.submit(Cmd::Fill { offset: 0, len: 1, value: 1 }).unwrap();
    assert!(g.submit(Cmd::Fill { offset: 0, len: 1, value: 1 }).is_err());
    let mut i = Input::new(1);
    i.push(Event::Press(Button::A)).unwrap();
    assert_eq!(i.push(Event::Press(Button::B)), Err(Error::Full));
    assert_eq!(i.dropped(), 1);
}

#[test]
fn driver_stop_restituisce_le_richieste_in_coda_come_fallite() {
    let mut d = Driver::new(m("d"));
    d.submit(Request { id: 4, op: 0 }).unwrap();
    d.stop();
    assert_eq!(d.take_completions(), vec![Completion { id: 4, result: Err(Error::Stopped) }]);
    let mut ds = vec![d];
    assert_eq!(tick(&mut ds), 0);
}
