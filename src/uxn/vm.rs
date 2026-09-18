use thiserror::Error;

use super::device::{NullDevice, VmDevice};
use super::memory::{MAXIMUM_ROM_SIZE, Memory};
use super::operands::Operands;
use super::stack::Stack;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum VmError {
    #[error("ROM size ({size} bytes) exceeds available RAM ({maximum_capacity} bytes)")]
    RomTooLarge {
        size: usize,
        maximum_capacity: usize,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum VmState {
    Ok,
    Halted,
}

pub struct Vm<D: VmDevice = NullDevice> {
    pub ws: Stack,
    pub rs: Stack,
    pub memory: Memory,
    pub device: D,
}

impl Vm<NullDevice> {
    pub fn new_with_rom(rom: &[u8]) -> Result<Self, VmError> {
        Self::new_with_rom_and_device(rom, NullDevice)
    }
}

impl<D: VmDevice> Vm<D> {
    pub fn new_with_rom_and_device(rom: &[u8], device: D) -> Result<Self, VmError> {
        if rom.len() > MAXIMUM_ROM_SIZE {
            return Err(VmError::RomTooLarge {
                size: rom.len(),
                maximum_capacity: MAXIMUM_ROM_SIZE,
            });
        }

        let mut memory = Memory::new();
        memory.load_rom(rom);

        Ok(Self {
            ws: Stack::default(),
            rs: Stack::default(),
            memory,
            device,
        })
    }

    /// Continuously executes instructions until the VM reaches a halted state (`BRK`).
    pub fn run(&mut self) -> Result<(), VmError> {
        while self.step()? == VmState::Ok {}
        Ok(())
    }

    /// Evaluates a vector by jumping to its address and running until halted (`BRK`).
    /// If `vector` is 0, this is a no-op and returns `Ok(())`.
    pub fn eval_vector(&mut self, vector: u16) -> Result<(), VmError> {
        if vector == 0 {
            return Ok(());
        }
        self.memory.jump_absolute(vector);
        self.run()
    }

    pub fn step(&mut self) -> Result<VmState, VmError> {
        let opcode = self.memory.read_u8();

        let is_return = (opcode & 0x40) != 0;
        let is_keep = (opcode & 0x80) != 0;

        let (src_stack, dst_stack) = if is_return {
            (&mut self.rs, &mut self.ws)
        } else {
            (&mut self.ws, &mut self.rs)
        };

        let operands = Operands::peek_from(src_stack);

        let opcode = if (opcode & 0x1f) != 0 {
            opcode & !(0x40 | 0x80)
        } else {
            opcode
        };

        match opcode {
            0x00 /* BRK */ => return Ok(VmState::Halted),
            0x01 /* INC */ => {
                src_stack.adjust_for_keep(1, is_keep);
                src_stack.push_u8(operands.a.wrapping_add(1));
            }
            0x02 /* POP */ => {
                src_stack.adjust_for_keep(1, is_keep);
            }
            0x03 /* NIP */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.a);
            }
            0x04 /* SWP */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.a);
                src_stack.push_u8(operands.b);
            }
            0x05 /* ROT */ => {
                src_stack.adjust_for_keep(3, is_keep);
                src_stack.push_u8(operands.b);
                src_stack.push_u8(operands.a);
                src_stack.push_u8(operands.c);
            }
            0x06 /* DUP */ => {
                src_stack.adjust_for_keep(1, is_keep);
                src_stack.push_u8(operands.a);
                src_stack.push_u8(operands.a);
            }
            0x07 /* OVR */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b);
                src_stack.push_u8(operands.a);
                src_stack.push_u8(operands.b);
            }
            0x08 /* EQU */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let equal = operands.b == operands.a;
                src_stack.push_u8(if equal { 0x01 } else { 0x00 });
            }
            0x09 /* NEQ */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let not_equal = operands.b != operands.a;
                src_stack.push_u8(if not_equal { 0x01 } else { 0x00 });
            }
            0x0A /* GTH */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let greater = operands.b > operands.a;
                src_stack.push_u8(if greater { 0x01 } else { 0x00 });
            }
            0x0B /* LTH */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let less = operands.b < operands.a;
                src_stack.push_u8(if less { 0x01 } else { 0x00 });
            }
            0x0C /* JMP */ => {
                src_stack.adjust_for_keep(1, is_keep);
                self.memory.jump_relative(operands.a as i8 as i16);
            }
            0x0D /* JCN */ => {
                src_stack.adjust_for_keep(2, is_keep);
                if operands.b != 0 {
                    self.memory.jump_relative(operands.a as i8 as i16);
                }
            }
            0x0E /* JSR */ => {
                src_stack.adjust_for_keep(1, is_keep);
                dst_stack.push_u16(self.memory.pc());
                self.memory.jump_relative(operands.a as i8 as i16);
            }
            0x0F /* STH */ => {
                src_stack.adjust_for_keep(1, is_keep);
                dst_stack.push_u8(operands.a);
            }
            0x10 /* LDZ */ => {
                src_stack.adjust_for_keep(1, is_keep);
                src_stack.push_u8(self.memory.read_u8_at(operands.a as u16));
            }
            0x11 /* STZ */ => {
                src_stack.adjust_for_keep(2, is_keep);
                self.memory.write_u8_at(operands.a as u16, operands.b);
            }
            0x12 /* LDR */ => {
                src_stack.adjust_for_keep(1, is_keep);
                let address = self.memory.pc().wrapping_add_signed(operands.a as i8 as i16);
                src_stack.push_u8(self.memory.read_u8_at(address));
            }
            0x13 /* STR */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let address = self.memory.pc().wrapping_add_signed(operands.a as i8 as i16);
                self.memory.write_u8_at(address, operands.b);
            }
            0x14 /* LDA */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(self.memory.read_u8_at(operands.a2));
            }
            0x15 /* STA */ => {
                src_stack.adjust_for_keep(3, is_keep);
                self.memory.write_u8_at(operands.a2, operands.c);
            }
            0x16 /* DEI */ => {
                src_stack.adjust_for_keep(1, is_keep);
                src_stack.push_u8(self.device.input(operands.a));
            }
            0x17 /* DEO */ => {
                src_stack.adjust_for_keep(2, is_keep);
                self.device
                    .output(operands.a, operands.b, &mut self.memory);
            }
            0x18 /* ADD */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b.wrapping_add(operands.a));
            }
            0x19 /* SUB */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b.wrapping_sub(operands.a));
            }
            0x1A /* MUL */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b.wrapping_mul(operands.a));
            }
            0x1B /* DIV */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(if operands.a != 0 {
                    operands.b.saturating_div(operands.a)
                } else {
                    0
                });
            }
            0x1C /* AND */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b & operands.a);
            }
            0x1D /* OR */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b | operands.a);
            }
            0x1E /* EOR */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u8(operands.b ^ operands.a);
            }
            0x1F /* SFT */ => {
                src_stack.adjust_for_keep(2, is_keep);
                let shift = operands.a;
                let right_shift = (shift & 0x0f) as u32;
                let left_shift = (shift >> 4) as u32;
                let value = operands.b.checked_shr(right_shift).unwrap_or(0);
                let value = value.checked_shl(left_shift).unwrap_or(0);
                src_stack.push_u8(value);
            }
            0x20 /* JCI */ => {
                let condition = self.ws.pop_u8();
                let offset = self.memory.read_i16();
                if condition != 0 {
                    self.memory.jump_relative(offset);
                }
                return Ok(VmState::Ok);
            }
            0x21 /* INC2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u16(operands.a2.wrapping_add(1));
            }
            0x22 /* POP2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
            }
            0x23 /* NIP2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.a2);
            }
            0x24 /* SWP2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.a2);
                src_stack.push_u16(operands.b2);
            }
            0x25 /* ROT2 */ => {
                src_stack.adjust_for_keep(6, is_keep);
                src_stack.push_u16(operands.b2);
                src_stack.push_u16(operands.a2);
                src_stack.push_u16(operands.c2);
            }
            0x26 /* DUP2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u16(operands.a2);
                src_stack.push_u16(operands.a2);
            }
            0x27 /* OVR2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2);
                src_stack.push_u16(operands.a2);
                src_stack.push_u16(operands.b2);
            }
            0x28 /* EQU2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                let equal = operands.b2 == operands.a2;
                src_stack.push_u8(if equal { 0x01 } else { 0x00 });
            }
            0x29 /* NEQ2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                let not_equal = operands.b2 != operands.a2;
                src_stack.push_u8(if not_equal { 0x01 } else { 0x00 });
            }
            0x2A /* GTH2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                let greater = operands.b2 > operands.a2;
                src_stack.push_u8(if greater { 0x01 } else { 0x00 });
            }
            0x2B /* LTH2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                let less = operands.b2 < operands.a2;
                src_stack.push_u8(if less { 0x01 } else { 0x00 });
            }
            0x2C /* JMP2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                self.memory.jump_absolute(operands.a2);
            }
            0x2D /* JCN2 */ => {
                src_stack.adjust_for_keep(3, is_keep);
                if operands.c != 0 {
                    self.memory.jump_absolute(operands.a2);
                }
            }
            0x2E /* JSR2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                dst_stack.push_u16(self.memory.pc());
                self.memory.jump_absolute(operands.a2);
            }
            0x2F /* STH2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                dst_stack.push_u16(operands.a2);
            }
            0x30 /* LDZ2 */ => {
                src_stack.adjust_for_keep(1, is_keep);
                let high_byte = self.memory.read_u8_at(operands.a as u16);
                let low_byte = self.memory.read_u8_at(operands.a.wrapping_add(1) as u16);
                src_stack.push_u8(high_byte);
                src_stack.push_u8(low_byte);
            }
            0x31 /* STZ2 */ => {
                src_stack.adjust_for_keep(3, is_keep);
                self.memory.write_u8_at(
                    operands.a as u16,
                    (operands.short_after_byte >> 8) as u8,
                );
                self.memory.write_u8_at(
                    operands.a.wrapping_add(1) as u16,
                    operands.short_after_byte as u8,
                );
            }
            0x32 /* LDR2 */ => {
                src_stack.adjust_for_keep(1, is_keep);
                let address = self.memory.pc().wrapping_add_signed(operands.a as i8 as i16);
                src_stack.push_u16(self.memory.read_u16_at(address));
            }
            0x33 /* STR2 */ => {
                src_stack.adjust_for_keep(3, is_keep);
                let address = self.memory.pc().wrapping_add_signed(operands.a as i8 as i16);
                self.memory.write_u16_at(address, operands.short_after_byte);
            }
            0x34 /* LDA2 */ => {
                src_stack.adjust_for_keep(2, is_keep);
                src_stack.push_u16(self.memory.read_u16_at(operands.a2));
            }
            0x35 /* STA2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                self.memory.write_u16_at(operands.a2, operands.b2);
            }
            0x36 /* DEI2 */ => {
                src_stack.adjust_for_keep(1, is_keep);
                let high_byte = self.device.input(operands.a);
                let low_byte = self.device.input(operands.a.wrapping_add(1));
                src_stack.push_u8(high_byte);
                src_stack.push_u8(low_byte);
            }
            0x37 /* DEO2 */ => {
                src_stack.adjust_for_keep(3, is_keep);
                self.device.output(
                    operands.a,
                    (operands.short_after_byte >> 8) as u8,
                    &mut self.memory,
                );
                self.device.output(
                    operands.a.wrapping_add(1),
                    operands.short_after_byte as u8,
                    &mut self.memory,
                );
            }
            0x38 /* ADD2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2.wrapping_add(operands.a2));
            }
            0x39 /* SUB2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2.wrapping_sub(operands.a2));
            }
            0x3A /* MUL2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2.wrapping_mul(operands.a2));
            }
            0x3B /* DIV2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(if operands.a2 != 0 {
                    operands.b2.saturating_div(operands.a2)
                } else {
                    0
                });
            }
            0x3C /* AND2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2 & operands.a2);
            }
            0x3D /* OR2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2 | operands.a2);
            }
            0x3E /* EOR2 */ => {
                src_stack.adjust_for_keep(4, is_keep);
                src_stack.push_u16(operands.b2 ^ operands.a2);
            }
            0x3F /* SFT2 */ => {
                src_stack.adjust_for_keep(3, is_keep);
                let shift = operands.a;
                let right_shift = (shift & 0x0f) as u32;
                let left_shift = (shift >> 4) as u32;
                let value = operands.short_after_byte.checked_shr(right_shift).unwrap_or(0);
                let value = value.checked_shl(left_shift).unwrap_or(0);
                src_stack.push_u16(value);
            }
            0x40 /* JMI */ => {
                let offset = self.memory.read_i16();
                self.memory.jump_relative(offset);
                return Ok(VmState::Ok);
            }
            0x60 /* JSI */ => {
                let offset = self.memory.read_i16();
                self.rs.push_u16(self.memory.pc());
                self.memory.jump_relative(offset);
                return Ok(VmState::Ok);
            }
            0x80 /* LIT */ => {
                self.ws.push_u8(self.memory.read_u8());
                return Ok(VmState::Ok);
            }
            0xa0 /* LIT2 */ => {
                self.ws.push_u16(self.memory.read_u16());
                return Ok(VmState::Ok);
            }
            0xc0 /* LITr */ => {
                self.rs.push_u8(self.memory.read_u8());
                return Ok(VmState::Ok);
            }
            0xe0 /* LIT2r */ => {
                self.rs.push_u16(self.memory.read_u16());
                return Ok(VmState::Ok);
            }
            _ => {}
        }

        Ok(VmState::Ok)
    }
}
