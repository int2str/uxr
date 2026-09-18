pub enum Number {
    Byte(u8),
    Short(u16),
}

impl Number {
    pub fn from_hex(hex_string: &str) -> Option<Self> {
        let is_hex = hex_string.chars().all(is_nibble);
        if !is_hex {
            return None;
        }

        match hex_string.len() {
            2 => Some(Number::Byte(u8::from_str_radix(hex_string, 16).ok()?)),
            4 => Some(Number::Short(u16::from_str_radix(hex_string, 16).ok()?)),
            _ => None,
        }
    }

    pub fn from_hex_padding(hex_string: &str) -> Option<u16> {
        if hex_string.is_empty() || hex_string.len() > 4 || !hex_string.chars().all(is_nibble) {
            return None;
        }
        u16::from_str_radix(hex_string, 16).ok()
    }

    pub fn as_short(&self) -> u16 {
        match *self {
            Self::Byte(value) => value as u16,
            Self::Short(value) => value,
        }
    }
}

fn is_nibble(chr: char) -> bool {
    matches!(chr, '0'..='9' | 'a'..='f')
}
