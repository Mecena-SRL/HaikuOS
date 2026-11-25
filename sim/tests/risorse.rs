use haiku_alloc_sim::resource::*;

const ALL: &[Arch] = &[Arch::Gba, Arch::Arm64, Arch::X86_64];
const ARM_X86: &[Arch] = &[Arch::Arm64, Arch::X86_64];

fn m(name: &'static str, archs: &'static [Arch], required: bool, bytes: usize) -> Manifest {
    Manifest { name, kind: Kind::Gpu, archs, required, bytes, queue_depth: 2 }
}

fn t(arch: Option<Arch>, ram: usize) -> Target {
    Target { arch, ram_free: ram }
}

#[test]
fn opzionale_incompatibile_saltato() {
    let ms = [m("cpu", ALL, true, 10), m("gpu-nvidia", ARM_X86, false, 10)];
    let p = plan(t(Some(Arch::Gba), 100), &ms).unwrap();
    assert_eq!(p.active, vec!["cpu"]);
    assert_eq!(p.skipped, vec!["gpu-nvidia"]);
}

#[test]
fn richiesto_incompatibile_blocca_tutto() {
    let ms = [m("cpu", ALL, true, 10), m("cpu-x86", ARM_X86, true, 10)];
    assert_eq!(
        plan(t(Some(Arch::Gba), 100), &ms),
        Err(PlanError::RequiredIncompatible("cpu-x86"))
    );
}

#[test]
fn ram_insufficiente_blocca_tutto() {
    let ms = [m("cpu", ALL, true, 60)];
    assert_eq!(
        plan(t(Some(Arch::Gba), 50), &ms),
        Err(PlanError::NotEnoughRam { need: 60, have: 50 })
    );
}

#[test]
fn architettura_sconosciuta_non_carica_nulla() {
    let ms = [m("cpu", ALL, true, 10)];
    assert_eq!(plan(t(None, 100), &ms), Err(PlanError::UnknownArch));
}

#[test]
fn attivazione_fallita_fa_rollback_in_ordine_inverso() {
    let ms = [m("a", ALL, true, 1), m("b", ALL, true, 1), m("c", ALL, true, 1)];
    let err = activate(&ms, |x| x.name == "c").unwrap_err();
    assert_eq!(err.failed, "c");
    assert_eq!(err.rolled_back, vec!["b", "a"]);
}

#[test]
fn due_gpu_lavorano_in_parallelo_senza_mescolare_i_risultati() {
    let ms = [m("gpu-cpu", ALL, true, 1), m("gpu-amd", ARM_X86, false, 1)];
    let mut ds = activate(&ms, |_| false).unwrap();
    ds[0].submit(Request { id: 1, op: 10 }).unwrap();
    ds[0].submit(Request { id: 2, op: 20 }).unwrap();
    ds[1].submit(Request { id: 7, op: 70 }).unwrap();

    assert_eq!(tick(&mut ds), 2); // un passo per driver nello stesso giro
    assert_eq!(ds[1].take_completions(), vec![Completion { id: 7, result: Ok(71) }]);
    assert_eq!(tick(&mut ds), 1);
    assert_eq!(
        ds[0].take_completions(),
        vec![
            Completion { id: 1, result: Ok(11) },
            Completion { id: 2, result: Ok(21) },
        ]
    );
}

#[test]
fn coda_piena_rifiuta() {
    let mut d = Driver::new(m("gpu", ALL, true, 1));
    d.submit(Request { id: 1, op: 0 }).unwrap();
    d.submit(Request { id: 2, op: 0 }).unwrap();
    assert_eq!(d.submit(Request { id: 3, op: 0 }), Err(Error::Full));
}

#[test]
fn stop_di_un_driver_non_tocca_gli_altri() {
    let ms = [m("a", ALL, true, 1), m("b", ALL, true, 1)];
    let mut ds = activate(&ms, |_| false).unwrap();
    ds[0].submit(Request { id: 1, op: 0 }).unwrap();
    ds[0].stop();
    assert_eq!(ds[0].take_completions(), vec![Completion { id: 1, result: Err(Error::Stopped) }]);
    assert_eq!(ds[0].submit(Request { id: 2, op: 0 }), Err(Error::Stopped));
    ds[1].submit(Request { id: 9, op: 0 }).unwrap();
    tick(&mut ds);
    assert_eq!(ds[1].take_completions(), vec![Completion { id: 9, result: Ok(1) }]);
}
