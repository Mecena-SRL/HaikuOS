use haiku_alloc_sim::audio::{AudioError, Mixer};
use haiku_alloc_sim::cpu::{AdmitError, Cpu, TaskReq};
use haiku_alloc_sim::net::{encode_frame, NetError, NetLink, BROADCAST, FRAME_MAX};
use haiku_alloc_sim::resource::{Arch, Driver, Kind, Manifest, Microkernel, Status};
use haiku_alloc_sim::swap::{Swap, SwapError, PAGE_BYTES};
use haiku_alloc_sim::crypto::Vault;

const MAC: [u8; 6] = [2, 0, 0, 0, 0, 1];
const OTHER: [u8; 6] = [2, 0, 0, 0, 0, 2];

#[test]
fn cpu_ammette_fino_al_limite_poi_si_ferma() {
    let mut c = Cpu::new(Arch::X86_64, 2);
    let t = TaskReq { bits: 64, needs_mmu: true };
    assert_eq!(c.admit_task(t), Ok(1));
    assert_eq!(c.admit_task(t), Ok(2));
    assert_eq!(c.admit_task(t), Err(AdmitError::Full));
    c.release_task();
    assert_eq!(c.admit_task(t), Ok(2));
    c.halt();
    assert_eq!(c.status(), Status::Stopped);
    assert_eq!(c.admit_task(t), Err(AdmitError::Halted));
}

#[test]
fn cpu_gba_rifiuta_programma_64_bit() {
    let mut c = Cpu::new(Arch::Gba, 4);
    assert_eq!(c.admit_task(TaskReq { bits: 64, needs_mmu: false }), Err(AdmitError::WordSize));
    assert_eq!(c.caps().bits, 32);
}

fn raw(dst: [u8; 6], src: [u8; 6], et: u16, payload: &[u8]) -> Vec<u8> {
    let mut b = [0u8; FRAME_MAX];
    let n = encode_frame(&mut b, &dst, &src, et, payload).unwrap();
    b[..n].to_vec()
}

#[test]
fn rete_filtra_per_mac_e_accetta_broadcast() {
    let mut n = NetLink::new(MAC, 4);
    let mine = raw(MAC, OTHER, 0x0800, &[1]);
    let other = raw(OTHER, OTHER, 0x0800, &[2]);
    let bc = raw(BROADCAST, OTHER, 0x0806, &[3]);
    assert_eq!(n.receive(&mine), Ok(true));
    assert_eq!(n.receive(&other), Ok(false));
    assert_eq!(n.receive(&bc), Ok(true));
    let mut out = [0u8; FRAME_MAX];
    let l = n.next_rx(&mut out).unwrap();
    assert_eq!(&out[14..l], &[1]);
    let l = n.next_rx(&mut out).unwrap();
    assert_eq!(&out[14..l], &[3]);
    assert!(n.next_rx(&mut out).is_none());
}

#[test]
fn rete_coda_piena_e_link_giu() {
    let mut n = NetLink::new(MAC, 1);
    let mut out = [0u8; FRAME_MAX];
    n.send(&MAC, &OTHER, 0, &[]).unwrap();
    assert_eq!(n.send(&MAC, &OTHER, 0, &[]), Err(NetError::Full));
    assert!(n.next_tx(&mut out).is_some());
    n.set_link(false);
    assert_eq!(n.status(), Status::Stopped);
    assert_eq!(n.send(&MAC, &OTHER, 0, &[]), Err(NetError::Down));
}

#[test]
fn rete_rifiuta_frame_oltre_mtu_prima_di_accodare() {
    let mut n = NetLink::new(MAC, 2);
    let mut out = [0u8; FRAME_MAX];
    assert_eq!(n.send(&MAC, &OTHER, 0, &[0; 1501]), Err(NetError::TooLarge));
    assert!(n.next_tx(&mut out).is_none());
}

#[test]
fn audio_push_tutto_o_niente() {
    let mut m = Mixer::new(1, 3).unwrap();
    m.push(0, &[1, 2]).unwrap();
    assert_eq!(m.push(0, &[3, 4]), Err(AudioError::Full));
    assert_eq!(m.mix(2), vec![1, 2]); // il secondo blocco non è entrato a metà
}

#[test]
fn swap_blocchi_liberabili_e_doppia_liberazione() {
    let mut sw = Swap::new(1, Vault::new([1; 16], 1, 512));
    let b = sw.page_out(&vec![5u8; PAGE_BYTES]).unwrap();
    assert_eq!(sw.page_out(&vec![5u8; PAGE_BYTES]), Err(SwapError::NoFreeBlock));
    sw.free_page(b).unwrap();
    assert!(sw.page_out(&vec![5u8; PAGE_BYTES]).is_ok());
    // il blocco è di nuovo occupato: liberarlo due volte alla fine satura la coda
    assert!(sw.free_page(0).is_ok());
    assert_eq!(sw.free_page(0), Err(SwapError::NoFreeBlock));
}

#[test]
fn driver_status_segue_lo_stop() {
    let m = Manifest { name: "d", kind: Kind::Gpu, archs: &[Arch::Gba], required: true, bytes: 1, queue_depth: 1 };
    let mut d = Driver::new(m);
    assert_eq!(d.status(), Status::Ready);
    d.stop();
    assert_eq!(d.status(), Status::Stopped);
}
