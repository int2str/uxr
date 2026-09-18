/// UXN VM stack with built-in stack pointer
pub struct Stack {
    pub(crate) data: [u8; 256],
    pub(crate) sp: u8,
}

impl Stack {
    pub fn push_u8(&mut self, byte: u8) {
        self.data[self.sp as usize] = byte;
        self.sp = self.sp.wrapping_add(1);
    }

    pub fn push_u16(&mut self, value: u16) {
        self.push_u8((value >> 8) as u8);
        self.push_u8(value as u8);
    }

    pub fn pop_u8(&mut self) -> u8 {
        self.sp = self.sp.wrapping_sub(1);
        self.data[self.sp as usize]
    }

    pub fn peek_u8(&self) -> u8 {
        self.data[self.sp.wrapping_sub(1) as usize]
    }

    pub fn peek_u8_at(&self, offset: u8) -> u8 {
        self.data[self.sp.wrapping_sub(1).wrapping_sub(offset) as usize]
    }

    pub fn peek_u16_at(&self, offset: u8) -> u16 {
        let high_byte = self.peek_u8_at(offset.wrapping_add(1)) as u16;
        let low_byte = self.peek_u8_at(offset) as u16;
        (high_byte << 8) | low_byte
    }

    pub fn adjust_for_keep(&mut self, consumed_bytes: u8, is_keep: bool) {
        if !is_keep {
            self.sp = self.sp.wrapping_sub(consumed_bytes);
        }
    }

    pub fn dup_u8(&mut self) {
        let byte = self.data[self.sp.wrapping_sub(1) as usize];
        self.push_u8(byte);
    }

    pub fn pop_u16(&mut self) -> u16 {
        let low_byte = self.pop_u8() as u16;
        let high_byte = self.pop_u8() as u16;
        (high_byte << 8) | low_byte
    }

    pub fn peek_u16(&self) -> u16 {
        let high_byte = self.data[self.sp.wrapping_sub(2) as usize] as u16;
        let low_byte = self.data[self.sp.wrapping_sub(1) as usize] as u16;
        (high_byte << 8) | low_byte
    }

    pub fn dup_u16(&mut self) {
        let value = self.peek_u16();
        self.push_u16(value);
    }

    pub fn pointer(&self) -> u8 {
        self.sp
    }

    pub fn data(&self) -> &[u8; 256] {
        &self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.sp as usize]
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            data: [0; 256],
            sp: 0,
        }
    }
}
