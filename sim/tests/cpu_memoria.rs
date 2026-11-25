use haiku_alloc_sim::cpu::*;
use haiku_alloc_sim::memory::*;
use haiku_alloc_sim::resource::Arch;

#[test]
fn cpu_32bit_non_ammette_programma_64bit() {
    let gba = caps_of(Arch::Gba);
    assert_eq!(admit(gba, TaskReq { bits: 64, needs_mmu: false }), Err(AdmitError::WordSize));
    assert_eq!(admit(gba, TaskReq { bits: 32, needs_mmu: false }), Ok(()));
}

#[test]
fn cpu_senza_mmu_rifiuta_isolamento_hardware() {
    let gba = caps_of(Arch::Gba);
    assert_eq!(admit(gba, TaskReq { bits: 32, needs_mmu: true }), Err(AdmitError::NoMmu));
    let x86 = caps_of(Arch::X86_64);
    assert_eq!(admit(x86, TaskReq { bits: 64, needs_mmu: true }), Ok(()));
}

#[test]
fn memoria_quota_rispettata() {
    let mut m = MemoryKernel::new(4, 2, 16, 2);
    m.set_quota(0, 1);
    assert!(m.alloc(0, Pool::Ram).is_ok());
    assert_eq!(m.alloc(0, Pool::Ram), Err(MemError::OverQuota));
    // il secondo proprietario non ha quota
    assert_eq!(m.alloc(1, Pool::Ram), Err(MemError::OverQuota));
}

#[test]
fn vram_e_pool_separato_dalla_ram() {
    let mut m = MemoryKernel::new(1, 1, 16, 1);
    m.set_quota(0, 2);
    let r = m.alloc(0, Pool::Ram).unwrap();
    // la RAM è piena, la VRAM no
    assert_eq!(m.alloc(0, Pool::Ram), Err(MemError::NoSpace));
    assert!(m.alloc(0, Pool::Vram).is_ok());
    assert!(m.read(r, 0).is_ok());
}

#[test]
fn revoca_azzera_prima_del_riuso() {
    let mut m = MemoryKernel::new(1, 0, 4, 2);
    m.set_quota(0, 1);
    m.set_quota(1, 1);
    let h = m.alloc(0, Pool::Ram).unwrap();
    m.write(h, 0, 0, &[1, 2, 3, 4]).unwrap();
    m.revoke(h, 0).unwrap();
    // il blocco ora appartiene a un altro proprietario: non deve contenere dati vecchi
    let h2 = m.alloc(1, Pool::Ram).unwrap();
    assert_eq!(m.read(h2, 1).unwrap(), vec![0, 0, 0, 0]);
}

#[test]
fn altro_proprietario_non_legge_ne_revoca() {
    let mut m = MemoryKernel::new(2, 0, 4, 2);
    m.set_quota(0, 1);
    let h = m.alloc(0, Pool::Ram).unwrap();
    assert_eq!(m.read(h, 1), Err(MemError::NotOwner));
    assert_eq!(m.revoke(h, 1), Err(MemError::NotOwner));
    assert!(m.read(h, 0).is_ok());
}

#[test]
fn scrittura_fuori_blocco_rifiutata() {
    let mut m = MemoryKernel::new(1, 0, 4, 1);
    m.set_quota(0, 1);
    let h = m.alloc(0, Pool::Ram).unwrap();
    assert_eq!(m.write(h, 0, 2, &[1, 2, 3]), Err(MemError::BadHandle));
}
