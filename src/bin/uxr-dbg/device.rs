use uxr::uxn::device::VmDevice;
use uxr::uxn::memory::Memory;

/// A headless VM device that captures console output and manages system
/// operations for the debugger.
pub struct CaptureDevice {
    dev: [u8; 256],
    console_output: String,
    console_lines: Vec<String>,
    terminated: bool,
    exit_code: u8,
}

impl CaptureDevice {
    pub fn new() -> Self {
        Self {
            dev: [0; 256],
            console_output: String::new(),
            console_lines: vec![String::new()],
            terminated: false,
            exit_code: 0,
        }
    }

    pub fn console_lines(&self) -> &[String] {
        &self.console_lines
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    fn record_character(&mut self, byte: u8) {
        let character = byte as char;
        self.console_output.push(character);

        if character == '\n' {
            self.console_lines.push(String::new());
        } else if let Some(last_line) = self.console_lines.last_mut() {
            last_line.push(character);
        } else {
            self.console_lines.push(character.to_string());
        }
    }
}

impl VmDevice for CaptureDevice {
    fn input(&mut self, port: u8) -> u8 {
        self.dev[port as usize]
    }

    fn output(&mut self, port: u8, value: u8, memory: &mut Memory) {
        self.dev[port as usize] = value;

        match port {
            // System/expansion execution
            0x03 => {
                let descriptor_address = ((self.dev[0x02] as u16) << 8) | (self.dev[0x03] as u16);
                memory.expansion_execute(descriptor_address);
            }
            // System/state termination
            0x0f => {
                if value != 0 {
                    self.terminated = true;
                    self.exit_code = value & 0x7f;
                }
            }
            // Console stdout and stderr
            0x18 | 0x19 => {
                self.record_character(value);
            }
            _ => {}
        }
    }
}
