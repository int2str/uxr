use ratatui::style::{Color, Modifier, Style};
use uxr::uxntal::MnemonicCategory;

use super::app::ExecutionState;

/// Trait defining the color and style contract for the debugger UI.
pub trait ThemeTrait {
    /// Background color for main panes.
    fn background_color() -> Color;

    /// Background color for header and footer chrome bars.
    fn bar_background_color() -> Color;

    /// Border color for header and footer chrome bars.
    fn bar_border_color() -> Color;

    /// Style for pane borders based on active focus.
    fn border_style(active: bool) -> Style;

    /// Style for pane titles based on active focus.
    fn title_style(active: bool) -> Style;

    /// Border color for stack panes.
    fn stack_border_color() -> Color;

    /// Style for stack pane titles.
    fn stack_title_style() -> Style;

    /// Standard foreground text color.
    fn text_color() -> Color;

    /// Dimmed / muted text color for addresses, parentheses, and omission dots.
    fn muted_color() -> Color;

    /// Secondary text color for ASCII characters and raw bytes.
    fn subtext_color() -> Color;

    /// Text color for badges and inverted tags.
    fn badge_text_color() -> Color;

    /// Style for the header title badge (" uxr-dbg ").
    fn header_title_style() -> Style;

    /// Status badge color when execution is paused.
    fn status_paused_color() -> Color;

    /// Status badge color when execution is running.
    fn status_running_color() -> Color;

    /// Status badge color when execution has halted.
    fn status_halted_color() -> Color;

    /// Status badge color when a breakpoint has been hit.
    fn status_breakpoint_color() -> Color;

    /// Status badge background color based on execution state.
    fn status_color(state: ExecutionState) -> Color {
        match state {
            ExecutionState::Paused => Self::status_paused_color(),
            ExecutionState::Running => Self::status_running_color(),
            ExecutionState::Halted => Self::status_halted_color(),
            ExecutionState::Breakpoint(_) => Self::status_breakpoint_color(),
        }
    }

    /// Style for an execution status badge given its background color.
    fn status_badge_style(status_color: Color) -> Style {
        Style::default()
            .fg(Self::badge_text_color())
            .bg(status_color)
            .add_modifier(Modifier::BOLD)
    }

    /// Text color for the ROM name in the header.
    fn rom_name_color() -> Color;

    /// Text color for the Program Counter (PC) value in the header.
    fn pc_color() -> Color;

    /// Text color for the instruction step counter in the header.
    fn step_count_color() -> Color;

    /// Text color for the breakpoint counter in the header.
    fn breakpoint_color() -> Color;

    /// Color for breakpoint indicators in disassembly.
    fn breakpoint_marker_color() -> Color;

    /// Color for the active PC indicator in disassembly.
    fn pc_indicator_color() -> Color;

    /// Color for label addresses in disassembly.
    fn label_address_color() -> Color;

    /// Color for label names in disassembly.
    fn label_name_color() -> Color;

    /// Color for instruction addresses in disassembly.
    fn instruction_address_color() -> Color;

    /// Color for instruction raw hex bytes in disassembly.
    fn instruction_bytes_color() -> Color;

    /// Color mapped to an opcode category in disassembly.
    fn mnemonic_color(category: MnemonicCategory) -> Color;

    /// Color for jump target addresses in disassembly.
    fn jump_target_color() -> Color;

    /// Color for target symbols in disassembly.
    fn symbol_color() -> Color;

    /// Color for numeric literal values in disassembly.
    fn literal_color() -> Color;

    /// Background color for the currently selected line in disassembly.
    fn selected_line_background_color() -> Color;

    /// Background color for the current PC line in disassembly when not selected.
    fn pc_line_background_color() -> Color;

    /// Color for stack entry indices.
    fn stack_index_color() -> Color;

    /// Color for stack entry hex bytes.
    fn stack_byte_color() -> Color;

    /// Color for stack entry decimal values.
    fn stack_decimal_color() -> Color;

    /// Color for the top-of-stack marker.
    fn stack_top_marker_color() -> Color;

    /// Color for shortcut brackets in the footer.
    fn footer_bracket_color() -> Color;

    /// Color for shortcut keys in the footer.
    fn shortcut_key_color() -> Color;

    /// Color for shortcut descriptions in the footer.
    fn shortcut_description_color() -> Color;
}
