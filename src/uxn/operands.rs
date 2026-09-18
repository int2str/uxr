use super::stack::Stack;

/// Preloaded operand bytes and shorts peeked non-destructively from a UXN stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operands {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub a2: u16,
    pub b2: u16,
    pub c2: u16,
    pub short_after_byte: u16,
}

impl Operands {
    /// Peeks operand candidates from the top of the given stack without modifying the stack pointer.
    pub fn peek_from(stack: &Stack) -> Self {
        Self {
            a: stack.peek_u8_at(0),
            b: stack.peek_u8_at(1),
            c: stack.peek_u8_at(2),
            a2: stack.peek_u16_at(0),
            b2: stack.peek_u16_at(2),
            c2: stack.peek_u16_at(4),
            short_after_byte: stack.peek_u16_at(1),
        }
    }
}
