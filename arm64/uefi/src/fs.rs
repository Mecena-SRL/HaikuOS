//! File system: volume FAT del firmware UEFI (EFI_SIMPLE_FILE_SYSTEM_PROTOCOL). Niente driver proprio.
//! Legge la lista dei file della radice, legge e scrive file.

use core::ffi::c_void;

pub const NAME_MAX: usize = 12;

// EFI_SIMPLE_FILE_SYSTEM_PROTOCOL: OpenVolume a 8.
// EFI_FILE_PROTOCOL: Open 8, Close 16, Read 32, Write 40, SetPosition 56.
type OpenVolume = extern "efiapi" fn(*mut c_void, *mut *mut c_void) -> usize;
type FileOpen = extern "efiapi" fn(*mut c_void, *mut *mut c_void, *const u16, u64, u64) -> usize;
type FileClose = extern "efiapi" fn(*mut c_void) -> usize;
type FileRead = extern "efiapi" fn(*mut c_void, *mut usize, *mut u8) -> usize;
type FileWrite = extern "efiapi" fn(*mut c_void, *mut usize, *const u8) -> usize;
type SetPosition = extern "efiapi" fn(*mut c_void, u64) -> usize;

const MODE_READ: u64 = 0x1;
const MODE_READ_WRITE_CREATE: u64 = 0x8000_0000_0000_0003;
const ATTR_DIR: u64 = 0x10;

pub struct Fs {
    root: *mut c_void,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub name: [u8; NAME_MAX],
    pub len: usize,
}

impl Entry {
    pub const EMPTY: Entry = Entry { name: [0; NAME_MAX], len: 0 };

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.len]).unwrap_or("?")
    }
}

unsafe fn func<T: Copy>(h: *mut c_void, off: usize) -> T {
    *((h as *const u8).add(off) as *const T)
}

/// Converte un nome ASCII in UTF-16 terminato da zero. `buf` deve avere almeno `name.len() + 1` elementi.
fn utf16(name: &[u8], buf: &mut [u16]) {
    for (i, &b) in name.iter().enumerate() {
        buf[i] = b as u16;
    }
    buf[name.len()] = 0;
}

impl Fs {
    /// Apre la radice del primo volume FAT. `open_volume` è il puntatore alla funzione del protocollo.
    pub fn open(sfs: *mut c_void) -> Option<Fs> {
        if sfs.is_null() {
            return None;
        }
        let mut root: *mut c_void = core::ptr::null_mut();
        let ov: OpenVolume = unsafe { func(sfs, 8) };
        if ov(sfs, &mut root) != 0 || root.is_null() {
            return None;
        }
        Some(Fs { root })
    }

    /// Elenca i file (non le directory) della radice. Ritorna quanti ne ha messi in `out`.
    pub fn list(&self, out: &mut [Entry]) -> usize {
        let mut n = 0;
        unsafe {
            let set_pos: SetPosition = func(self.root, 56);
            let read: FileRead = func(self.root, 32);
            set_pos(self.root, 0);
            let mut buf = [0u8; 600];
            while n < out.len() {
                let mut size = buf.len();
                if read(self.root, &mut size, buf.as_mut_ptr()) != 0 || size == 0 {
                    break;
                }
                // EFI_FILE_INFO: Attribute u64 a 72, FileName (UTF-16, zero finale) a 80.
                let attr = u64::from_le_bytes(buf[72..80].try_into().unwrap_or([0; 8]));
                if attr & ATTR_DIR != 0 {
                    continue;
                }
                let mut e = Entry::EMPTY;
                let mut i = 0;
                let mut off = 80;
                while off + 1 < size {
                    let c = u16::from_le_bytes([buf[off], buf[off + 1]]);
                    if c == 0 || i == NAME_MAX {
                        break;
                    }
                    e.name[i] = if c < 128 { c as u8 } else { b'?' };
                    i += 1;
                    off += 2;
                }
                e.len = i;
                out[n] = e;
                n += 1;
            }
        }
        n
    }

    /// Legge un file della radice in `buf`. Ritorna i byte letti (0 se il file non c'è).
    pub fn read(&self, name: &[u8], buf: &mut [u8]) -> usize {
        let mut u = [0u16; NAME_MAX + 1];
        utf16(name, &mut u);
        unsafe {
            let open: FileOpen = func(self.root, 8);
            let close: FileClose = func(self.root, 16);
            let read: FileRead = func(self.root, 32);
            let mut f: *mut c_void = core::ptr::null_mut();
            if open(self.root, &mut f, u.as_ptr(), MODE_READ, 0) != 0 {
                return 0;
            }
            let mut total = 0;
            while total < buf.len() {
                let mut size = buf.len() - total;
                if read(f, &mut size, buf.as_mut_ptr().add(total)) != 0 || size == 0 {
                    break;
                }
                total += size;
            }
            close(f);
            total
        }
    }

    /// Scrive `data` nel file della radice, creandolo se manca. True se riesce.
    pub fn write(&self, name: &[u8], data: &[u8]) -> bool {
        let mut u = [0u16; NAME_MAX + 1];
        utf16(name, &mut u);
        unsafe {
            let open: FileOpen = func(self.root, 8);
            let close: FileClose = func(self.root, 16);
            let write: FileWrite = func(self.root, 40);
            let mut f: *mut c_void = core::ptr::null_mut();
            if open(self.root, &mut f, u.as_ptr(), MODE_READ_WRITE_CREATE, 0) != 0 {
                return false;
            }
            let mut size = data.len();
            let ok = write(f, &mut size, data.as_ptr()) == 0 && size == data.len();
            close(f);
            ok
        }
    }
}
