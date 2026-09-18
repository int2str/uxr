use super::location::Location;
use super::opcode::{OpCode, OpCodeModes};

/// The resolution kind for a label reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// `;label` -> LIT2 + short absolute address (3 bytes)
    LiteralAbsolute,
    /// `,label` -> LIT + relative byte offset (2 bytes)
    LiteralRelative,
    /// `.label` -> LIT + zero-page byte address (2 bytes)
    LiteralZeroPage,
    /// `=label` -> short absolute address (2 bytes)
    RawAbsolute,
    /// `_label` -> relative byte offset (1 byte)
    RawRelative,
    /// `-label` -> zero-page byte address (1 byte)
    RawZeroPage,
    /// `!label` -> JMI + relative short offset (3 bytes)
    JumpImmediate,
    /// `?label` -> JCI + relative short offset (3 bytes)
    JumpConditional,
    /// `label` -> JSI + relative short offset (3 bytes)
    JumpStash,
}

impl ReferenceKind {
    /// Returns the number of bytes this reference occupies in ROM.
    pub const fn byte_len(&self) -> u16 {
        match self {
            Self::RawRelative | Self::RawZeroPage => 1,
            Self::LiteralRelative | Self::LiteralZeroPage | Self::RawAbsolute => 2,
            Self::LiteralAbsolute
            | Self::JumpImmediate
            | Self::JumpConditional
            | Self::JumpStash => 3,
        }
    }

    /// Returns the leading rune character associated with this reference kind, if any.
    pub const fn rune(&self) -> Option<char> {
        match self {
            Self::LiteralAbsolute => Some(';'),
            Self::LiteralRelative => Some(','),
            Self::LiteralZeroPage => Some('.'),
            Self::RawAbsolute => Some('='),
            Self::RawRelative => Some('_'),
            Self::RawZeroPage => Some('-'),
            Self::JumpImmediate => Some('!'),
            Self::JumpConditional => Some('?'),
            Self::JumpStash => None,
        }
    }
}

/// The specific payload of a linear intermediate representation statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    // Memory Directives
    PadAbsolute(u16),
    PadRelative(u16),
    Label(String),
    Sublabel(String),

    // Instructions & Data
    Instruction(OpCode, OpCodeModes),
    LiteralByte(u8),
    LiteralShort(u16),
    RawBytes(Vec<u8>),

    // Unresolved References
    Reference { kind: ReferenceKind, target: String },
}

impl StatementKind {
    /// Returns the exact number of bytes this statement occupies in ROM.
    pub fn byte_len(&self) -> u16 {
        match self {
            Self::PadAbsolute(_) | Self::PadRelative(_) => 0,
            Self::Label(_) | Self::Sublabel(_) => 0,
            Self::Instruction(..) => 1,
            Self::LiteralByte(_) => 2,
            Self::LiteralShort(_) => 3,
            Self::RawBytes(bytes) => bytes.len() as u16,
            Self::Reference { kind, .. } => kind.byte_len(),
        }
    }

    /// Attaches a [`Location`] to this statement kind, producing a [`Statement`].
    pub fn at(self, location: Location) -> Statement {
        Statement::new(self, location)
    }

    pub fn is_memory_directive(&self) -> bool {
        matches!(
            self,
            Self::PadAbsolute(_) | Self::PadRelative(_) | Self::Label(_) | Self::Sublabel(_)
        )
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Self::Reference { .. })
    }
}

/// A linear intermediate representation statement associated with its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub location: Location,
}

impl Statement {
    pub fn new(kind: StatementKind, location: Location) -> Self {
        Self { kind, location }
    }

    /// Returns the exact number of bytes this statement will occupy in ROM.
    pub fn byte_len(&self) -> u16 {
        self.kind.byte_len()
    }
}
