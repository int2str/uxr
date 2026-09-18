#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OpCode {
    BRK,
    LIT,
    INC,
    POP,
    NIP,
    SWP,
    ROT,
    DUP,
    OVR,
    EQU,
    NEQ,
    GTH,
    LTH,
    JMP,
    JCN,
    JSR,
    STH,
    LDZ,
    STZ,
    LDR,
    STR,
    LDA,
    STA,
    DEI,
    DEO,
    ADD,
    SUB,
    MUL,
    DIV,
    AND,
    ORA,
    EOR,
    SFT,
}

impl OpCode {
    pub fn from_mnemonic(mnemonic: &str) -> Option<OpCode> {
        match mnemonic {
            "BRK" => Some(OpCode::BRK),
            "LIT" => Some(OpCode::LIT),
            "INC" => Some(OpCode::INC),
            "POP" => Some(OpCode::POP),
            "NIP" => Some(OpCode::NIP),
            "SWP" => Some(OpCode::SWP),
            "ROT" => Some(OpCode::ROT),
            "DUP" => Some(OpCode::DUP),
            "OVR" => Some(OpCode::OVR),
            "EQU" => Some(OpCode::EQU),
            "NEQ" => Some(OpCode::NEQ),
            "GTH" => Some(OpCode::GTH),
            "LTH" => Some(OpCode::LTH),
            "JMP" => Some(OpCode::JMP),
            "JCN" => Some(OpCode::JCN),
            "JSR" => Some(OpCode::JSR),
            "STH" => Some(OpCode::STH),
            "LDZ" => Some(OpCode::LDZ),
            "STZ" => Some(OpCode::STZ),
            "LDR" => Some(OpCode::LDR),
            "STR" => Some(OpCode::STR),
            "LDA" => Some(OpCode::LDA),
            "STA" => Some(OpCode::STA),
            "DEI" => Some(OpCode::DEI),
            "DEO" => Some(OpCode::DEO),
            "ADD" => Some(OpCode::ADD),
            "SUB" => Some(OpCode::SUB),
            "MUL" => Some(OpCode::MUL),
            "DIV" => Some(OpCode::DIV),
            "AND" => Some(OpCode::AND),
            "ORA" => Some(OpCode::ORA),
            "EOR" => Some(OpCode::EOR),
            "SFT" => Some(OpCode::SFT),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            OpCode::BRK => 0x00,
            OpCode::LIT => 0x80,
            OpCode::INC => 0x01,
            OpCode::POP => 0x02,
            OpCode::NIP => 0x03,
            OpCode::SWP => 0x04,
            OpCode::ROT => 0x05,
            OpCode::DUP => 0x06,
            OpCode::OVR => 0x07,
            OpCode::EQU => 0x08,
            OpCode::NEQ => 0x09,
            OpCode::GTH => 0x0a,
            OpCode::LTH => 0x0b,
            OpCode::JMP => 0x0c,
            OpCode::JCN => 0x0d,
            OpCode::JSR => 0x0e,
            OpCode::STH => 0x0f,
            OpCode::LDZ => 0x10,
            OpCode::STZ => 0x11,
            OpCode::LDR => 0x12,
            OpCode::STR => 0x13,
            OpCode::LDA => 0x14,
            OpCode::STA => 0x15,
            OpCode::DEI => 0x16,
            OpCode::DEO => 0x17,
            OpCode::ADD => 0x18,
            OpCode::SUB => 0x19,
            OpCode::MUL => 0x1a,
            OpCode::DIV => 0x1b,
            OpCode::AND => 0x1c,
            OpCode::ORA => 0x1d,
            OpCode::EOR => 0x1e,
            OpCode::SFT => 0x1f,
        }
    }

    pub fn from_str_with_modes(name: &str) -> Option<(OpCode, OpCodeModes)> {
        if name.len() < 3 {
            return None;
        }

        let base_opcode = Self::from_mnemonic(&name[..3])?;

        if name.len() == 3 {
            return Some((base_opcode, OpCodeModes::default()));
        }

        if base_opcode == OpCode::BRK || name.len() > 6 {
            return None;
        }

        let mut modes = OpCodeModes::default();
        name[3..]
            .chars()
            .all(|character| modes.insert(character))
            .then_some((base_opcode, modes))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct OpCodeModes(u8);

impl OpCodeModes {
    pub const SHORT: u8 = 0x20;
    pub const RETURN: u8 = 0x40;
    pub const KEEP: u8 = 0x80;

    pub fn insert(&mut self, chr: char) -> bool {
        let flag = match chr {
            '2' => Self::SHORT,
            'r' => Self::RETURN,
            'k' => Self::KEEP,
            _ => return false,
        };
        if self.0 & flag != 0 {
            return false;
        }
        self.0 |= flag;
        true
    }

    pub fn is_short(&self) -> bool {
        self.0 & Self::SHORT != 0
    }

    pub fn is_return(&self) -> bool {
        self.0 & Self::RETURN != 0
    }

    pub fn is_keep(&self) -> bool {
        self.0 & Self::KEEP != 0
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<OpCodeModes> for u8 {
    fn from(modes: OpCodeModes) -> Self {
        modes.0
    }
}

impl From<u8> for OpCodeModes {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}
