pub const RAM_SIZE: usize = 65_536;
pub const TOTAL_BANKS: usize = 16;
pub const EXPANSION_BANKS_COUNT: usize = TOTAL_BANKS - 1;
pub const MAXIMUM_ROM_SIZE: usize = RAM_SIZE - PROGRAM_START;

const PROGRAM_START: usize = 0x0100;

pub struct Memory {
    pc: u16,
    ram: [u8; RAM_SIZE],
    expansion_banks: Box<[u8]>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            pc: PROGRAM_START as u16,
            ram: [0; RAM_SIZE],
            expansion_banks: vec![0u8; EXPANSION_BANKS_COUNT * RAM_SIZE].into_boxed_slice(),
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.ram[PROGRAM_START..PROGRAM_START + rom.len()].copy_from_slice(rom);
    }

    pub fn read_u8(&mut self) -> u8 {
        let byte = self.ram[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    pub fn read_i8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    pub fn read_u8_at(&self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    pub fn write_u8_at(&mut self, address: u16, byte: u8) {
        self.ram[address as usize] = byte;
    }

    pub fn read_u16(&mut self) -> u16 {
        let high_byte = self.read_u8() as u16;
        let low_byte = self.read_u8() as u16;
        (high_byte << 8) | low_byte
    }

    pub fn read_i16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    pub fn read_u16_at(&self, address: u16) -> u16 {
        let high_byte = self.read_u8_at(address) as u16;
        let low_byte = self.read_u8_at(address.wrapping_add(1)) as u16;
        (high_byte << 8) | low_byte
    }

    pub fn write_u16_at(&mut self, address: u16, value: u16) {
        self.write_u8_at(address, (value >> 8) as u8);
        self.write_u8_at(address.wrapping_add(1), value as u8);
    }

    pub fn jump_relative(&mut self, offset: i16) {
        self.pc = self.pc.wrapping_add_signed(offset);
    }

    pub fn jump_absolute(&mut self, address: u16) {
        self.pc = address;
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn ram(&self) -> &[u8; RAM_SIZE] {
        &self.ram
    }

    pub fn ram_mut(&mut self) -> &mut [u8; RAM_SIZE] {
        &mut self.ram
    }

    /// Returns a reference to the 64 KB slice of the specified bank, if within bounds (0..16).
    pub fn bank(&self, bank_index: usize) -> Option<&[u8]> {
        if bank_index == 0 {
            Some(&self.ram)
        } else if bank_index < TOTAL_BANKS {
            let start = (bank_index - 1) * RAM_SIZE;
            Some(&self.expansion_banks[start..start + RAM_SIZE])
        } else {
            None
        }
    }

    /// Returns a mutable reference to the 64 KB slice of the specified bank, if within bounds (0..16).
    pub fn bank_mut(&mut self, bank_index: usize) -> Option<&mut [u8]> {
        if bank_index == 0 {
            Some(&mut self.ram)
        } else if bank_index < TOTAL_BANKS {
            let start = (bank_index - 1) * RAM_SIZE;
            Some(&mut self.expansion_banks[start..start + RAM_SIZE])
        } else {
            None
        }
    }

    /// Executes a Varvara System/expansion memory management or DMA operation.
    ///
    /// The operation descriptor is read from Bank 0 at `descriptor_address`.
    pub fn expansion_execute(&mut self, descriptor_address: u16) {
        let command = self.read_u8_at(descriptor_address);
        let requested_length = self.read_u16_at(descriptor_address.wrapping_add(1)) as usize;

        match command {
            0x00 => {
                let target_bank = self.read_u16_at(descriptor_address.wrapping_add(3));
                let target_address = self.read_u16_at(descriptor_address.wrapping_add(5));
                let fill_value = self.read_u8_at(descriptor_address.wrapping_add(7));

                self.expansion_fill(target_bank, target_address, requested_length, fill_value);
            }
            0x01 | 0x02 => {
                let source_bank = self.read_u16_at(descriptor_address.wrapping_add(3));
                let source_address = self.read_u16_at(descriptor_address.wrapping_add(5));
                let destination_bank = self.read_u16_at(descriptor_address.wrapping_add(7));
                let destination_address = self.read_u16_at(descriptor_address.wrapping_add(9));

                self.expansion_copy(
                    source_bank,
                    source_address,
                    destination_bank,
                    destination_address,
                    requested_length,
                );
            }
            _ => {
                // Unknown commands are ignored per Varvara reference behavior.
            }
        }
    }

    fn expansion_fill(
        &mut self,
        target_bank: u16,
        target_address: u16,
        requested_length: usize,
        fill_value: u8,
    ) {
        let bank_index = target_bank as usize;
        if bank_index >= TOTAL_BANKS {
            return;
        }

        let address = target_address as usize;
        let available = RAM_SIZE - address;
        let length = requested_length.min(available);
        if length == 0 {
            return;
        }

        if bank_index == 0 {
            self.ram[address..address + length].fill(fill_value);
        } else {
            let start = (bank_index - 1) * RAM_SIZE + address;
            self.expansion_banks[start..start + length].fill(fill_value);
        }
    }

    fn expansion_copy(
        &mut self,
        source_bank: u16,
        source_address: u16,
        destination_bank: u16,
        destination_address: u16,
        requested_length: usize,
    ) {
        let source_bank_index = source_bank as usize;
        let destination_bank_index = destination_bank as usize;
        if source_bank_index >= TOTAL_BANKS || destination_bank_index >= TOTAL_BANKS {
            return;
        }

        let source_offset = source_address as usize;
        let destination_offset = destination_address as usize;

        let source_available = RAM_SIZE - source_offset;
        let destination_available = RAM_SIZE - destination_offset;
        let length = requested_length
            .min(source_available)
            .min(destination_available);
        if length == 0 {
            return;
        }

        match (source_bank_index, destination_bank_index) {
            (0, 0) => {
                self.ram
                    .copy_within(source_offset..source_offset + length, destination_offset);
            }
            (0, destination_index) => {
                let destination_start = (destination_index - 1) * RAM_SIZE + destination_offset;
                self.expansion_banks[destination_start..destination_start + length]
                    .copy_from_slice(&self.ram[source_offset..source_offset + length]);
            }
            (source_index, 0) => {
                let source_start = (source_index - 1) * RAM_SIZE + source_offset;
                self.ram[destination_offset..destination_offset + length]
                    .copy_from_slice(&self.expansion_banks[source_start..source_start + length]);
            }
            (source_index, destination_index) => {
                let source_start = (source_index - 1) * RAM_SIZE + source_offset;
                let destination_start = (destination_index - 1) * RAM_SIZE + destination_offset;
                self.expansion_banks
                    .copy_within(source_start..source_start + length, destination_start);
            }
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
