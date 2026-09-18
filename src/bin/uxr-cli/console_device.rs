use std::io::{self, Read, Write};

use uxr::uxn::device::VmDevice;
use uxr::uxn::memory::Memory;

pub struct ConsoleDevice {
    device: [u8; 256],
}

impl ConsoleDevice {
    pub fn new() -> Self {
        Self { device: [0; 256] }
    }
}

impl VmDevice for ConsoleDevice {
    fn input(&mut self, port: u8) -> u8 {
        match port {
            // Console/read
            0x12 => {
                let mut buffer = [0u8; 1];
                match io::stdin().lock().read_exact(&mut buffer) {
                    Ok(()) => buffer[0],
                    Err(_) => 0,
                }
            }
            _ => self.device[port as usize],
        }
    }

    fn output(&mut self, port: u8, value: u8, memory: &mut Memory) {
        self.device[port as usize] = value;

        match port {
            // System/expansion
            0x03 => {
                let descriptor_address =
                    ((self.device[0x02] as u16) << 8) | (self.device[0x03] as u16);
                memory.expansion_execute(descriptor_address);
            }
            // System/state
            0x0f => {
                if value != 0 {
                    let exit_code = value & 0x7f;
                    std::process::exit(exit_code as i32);
                }
            }
            // Console/write
            0x18 => {
                let mut stdout = io::stdout().lock();
                let _ = stdout.write_all(&[value]);
                let _ = stdout.flush();
            }
            // Console/error
            0x19 => {
                let mut stderr = io::stderr().lock();
                let _ = stderr.write_all(&[value]);
                let _ = stderr.flush();
            }
            0x00..=0x0e | 0x10..=0x17 | 0x1a..=0x1f => {
                // Values written to ports other than the console ports and
                // relevant system ports are just stored and can be retrieved
                // through DEI.
            }
            _ => {
                eprintln!("Unknown write to port 0x{port:02x} -> 0x{value:02x}");
            }
        }
    }
}
