use super::location::Location;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenKind {
    Include,
    BraceOpen,    // {
    BraceClose,   // }
    BracketOpen,  // [
    BracketClose, // ]
    MacroDefinition,
    Number,
    HexLiteral,
    OpCode,
    RawString,
    Identifier,
    PadAbsolute,
    PadRelative,
    Label,
    Sublabel,
    JumpImmediate,
    JumpConditional,
    LiteralAbsolute,
    LiteralZeroPage,
    LiteralRelative,
    RawAbsolute,
    RawZeroPage,
    RawRelative,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub location: Location,
}
