use haiku_alloc_sim::gpu::*;
use haiku_alloc_sim::net::*;
use haiku_alloc_sim::storage::*;

#[test]
fn storage_round_trip() {
    let mut d = Device::new(2, 4);
    d.write(1, &[1, 2, 3, 4]).unwrap();
    assert_eq!(d.read(1).unwrap(), &[1, 2, 3, 4]);
}

#[test]
fn storage_rileva_blocco_alterato() {
    let mut d = Device::new(1, 4);
    d.write(0, &[9, 9, 9, 9]).unwrap();
    d.inject_fault(0, 2);
    assert_eq!(d.read(0), Err(StorageError::Corrupt));
}

#[test]
fn storage_limiti_e_dimensione() {
    let mut d = Device::new(1, 4);
    assert_eq!(d.write(1, &[0; 4]), Err(StorageError::OutOfRange));
    assert_eq!(d.write(0, &[0; 3]), Err(StorageError::BadSize));
    assert_eq!(d.read(5), Err(StorageError::OutOfRange));
}

#[test]
fn gpu_due_istanze_hanno_vram_separate() {
    let mut a = Gpu::new(8, 4);
    let mut b = Gpu::new(8, 4);
    a.submit(Cmd::Fill { offset: 0, len: 4, value: 7 }).unwrap();
    a.run().unwrap();
    assert_eq!(&a.vram()[..4], &[7, 7, 7, 7]);
    assert_eq!(&b.vram()[..4], &[0, 0, 0, 0]);
    b.submit(Cmd::Fill { offset: 4, len: 4, value: 1 }).unwrap();
    b.run().unwrap();
    assert_eq!(&a.vram()[4..], &[0, 0, 0, 0]);
}

#[test]
fn gpu_comando_fuori_vram_senza_effetti() {
    let mut g = Gpu::new(4, 4);
    g.submit(Cmd::Fill { offset: 2, len: 4, value: 5 }).unwrap();
    assert_eq!(g.run(), Err(GpuError::OutOfVram));
    assert_eq!(g.vram(), &[0, 0, 0, 0]);
}

#[test]
fn gpu_coda_piena_rifiuta() {
    let mut g = Gpu::new(4, 1);
    g.submit(Cmd::Fill { offset: 0, len: 1, value: 1 }).unwrap();
    assert_eq!(
        g.submit(Cmd::Fill { offset: 0, len: 1, value: 1 }),
        Err(GpuError::Full)
    );
}

#[test]
fn frame_round_trip() {
    let mut b = [0u8; FRAME_MAX];
    let n = encode_frame(&mut b, &[1; 6], &[2; 6], 0x0800, &[3, 4, 5]).unwrap();
    let f = FrameRef::parse(&b[..n]).unwrap();
    assert_eq!((f.dst, f.src, f.ethertype, f.payload), ([1; 6], [2; 6], 0x0800, &[3u8, 4, 5][..]));
}

#[test]
fn frame_mtu_e_lunghezza_minima() {
    let mut b = [0u8; FRAME_MAX + 1];
    assert_eq!(encode_frame(&mut b, &[0; 6], &[0; 6], 0, &[0; MTU + 1]), Err(NetError::TooLarge));
    assert_eq!(FrameRef::parse(&[0; 13]), Err(NetError::TooShort));
}

#[test]
fn checksum_ipv4_esempio_rfc1071() {
    // Intestazione IPv4 d'esempio con checksum a 0 (valore atteso 0xb861).
    let hdr = [
        0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0xc0, 0xa8, 0x00,
        0x01, 0xc0, 0xa8, 0x00, 0xc7,
    ];
    assert_eq!(internet_checksum(&hdr), 0xb861);
}
