//! Test host: il driver di `src/driver.rs` gira contro un dispositivo finto.
use driverkit::bus::Bus;
use driverkit::driver::{Driver, Error, REG_CMD, REG_DATA, REG_STATUS, ST_BUSY, ST_DATA_READY};
use std::collections::VecDeque;

/// Dispositivo finto: FIFO di byte, log dei comandi, occupazione che si esaurisce con `wait`.
#[derive(Default)]
struct FakeDev {
    fifo: VecDeque<u8>,
    cmds: Vec<u8>,
    busy_polls: u32,
}

impl Bus for FakeDev {
    fn read_reg(&mut self, offset: u32) -> u32 {
        match offset {
            REG_STATUS => {
                let mut s = 0;
                if !self.fifo.is_empty() {
                    s |= ST_DATA_READY;
                }
                if self.busy_polls > 0 {
                    s |= ST_BUSY;
                }
                s
            }
            REG_DATA => self.fifo.pop_front().unwrap_or(0) as u32,
            _ => 0,
        }
    }

    fn write_reg(&mut self, offset: u32, value: u32) {
        if offset == REG_CMD {
            self.cmds.push(value as u8);
        }
    }

    fn wait(&mut self) {
        self.busy_polls = self.busy_polls.saturating_sub(1);
    }
}

#[test]
fn read_con_dato_restituisce_il_byte() {
    let mut dev = FakeDev::default();
    dev.fifo.extend([0x41, 0x42]);
    let mut drv = Driver::new(dev);
    assert_eq!(drv.read_byte(), Some(0x41));
    assert_eq!(drv.read_byte(), Some(0x42));
    assert_eq!(drv.read_byte(), None);
}

#[test]
fn read_senza_dato_restituisce_none() {
    let mut drv = Driver::new(FakeDev::default());
    assert_eq!(drv.read_byte(), None);
    assert!(drv.bus().fifo.is_empty());
}

#[test]
fn send_cmd_occupato_restituisce_err() {
    let dev = FakeDev { busy_polls: 1000, ..Default::default() };
    let mut drv = Driver::new(dev);
    assert_eq!(drv.send_cmd(0x01), Err(Error::Busy));
    assert!(drv.bus().cmds.is_empty());
}

#[test]
fn send_cmd_attende_e_scrive_quando_libero() {
    let dev = FakeDev { busy_polls: 2, ..Default::default() };
    let mut drv = Driver::new(dev);
    assert_eq!(drv.send_cmd(0x07), Ok(()));
    assert_eq!(drv.bus().cmds, vec![0x07]);
}
