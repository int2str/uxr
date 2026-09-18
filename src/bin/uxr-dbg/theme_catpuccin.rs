use ratatui::style::{Color, Modifier, Style};

use uxr::uxntal::MnemonicCategory;

use super::theme::ThemeTrait;

/// Catppuccin Mocha color palette constants.
///
/// These are the 26 canonical color names defined by the official
/// [Catppuccin specification](https://github.com/catppuccin/catppuccin).
///
/// The Catpuccin theme is available under the MIT license.
pub struct Catppuccin;

#[allow(dead_code)]
impl Catppuccin {
    pub const ROSEWATER: Color = Color::Rgb(0xf5, 0xe0, 0xdc);
    pub const FLAMINGO: Color = Color::Rgb(0xf2, 0xcd, 0xcd);
    pub const PINK: Color = Color::Rgb(0xf5, 0xc2, 0xe7);
    pub const MAUVE: Color = Color::Rgb(0xcb, 0xa6, 0xf7);
    pub const RED: Color = Color::Rgb(0xf3, 0x8b, 0xa8);
    pub const MAROON: Color = Color::Rgb(0xeb, 0xa0, 0xac);
    pub const PEACH: Color = Color::Rgb(0xfa, 0xb3, 0x87);
    pub const YELLOW: Color = Color::Rgb(0xf9, 0xe2, 0xaf);
    pub const GREEN: Color = Color::Rgb(0xa6, 0xe3, 0xa1);
    pub const TEAL: Color = Color::Rgb(0x94, 0xe2, 0xd5);
    pub const SKY: Color = Color::Rgb(0x89, 0xdc, 0xeb);
    pub const SAPPHIRE: Color = Color::Rgb(0x74, 0xc7, 0xec);
    pub const BLUE: Color = Color::Rgb(0x89, 0xb4, 0xfa);
    pub const LAVENDER: Color = Color::Rgb(0xb4, 0xbe, 0xfe);
    pub const TEXT: Color = Color::Rgb(0xcd, 0xd6, 0xf4);
    pub const SUBTEXT1: Color = Color::Rgb(0xba, 0xc2, 0xde);
    pub const SUBTEXT0: Color = Color::Rgb(0xa6, 0xad, 0xc8);
    pub const OVERLAY2: Color = Color::Rgb(0x93, 0x99, 0xb2);
    pub const OVERLAY1: Color = Color::Rgb(0x7f, 0x84, 0x9c);
    pub const OVERLAY0: Color = Color::Rgb(0x6c, 0x70, 0x86);
    pub const SURFACE2: Color = Color::Rgb(0x58, 0x5b, 0x70);
    pub const SURFACE1: Color = Color::Rgb(0x45, 0x47, 0x5a);
    pub const SURFACE0: Color = Color::Rgb(0x31, 0x32, 0x44);
    pub const BASE: Color = Color::Rgb(0x1e, 0x1e, 0x2e);
    pub const MANTLE: Color = Color::Rgb(0x18, 0x18, 0x25);
    pub const CRUST: Color = Color::Rgb(0x11, 0x11, 0x1b);
}

/// Catppuccin Mocha theme implementation of [`ThemeTrait`].
pub struct CatppuccinTheme;

impl ThemeTrait for CatppuccinTheme {
    fn background_color() -> Color {
        Catppuccin::BASE
    }

    fn bar_background_color() -> Color {
        Catppuccin::MANTLE
    }

    fn bar_border_color() -> Color {
        Catppuccin::SURFACE1
    }

    fn border_style(active: bool) -> Style {
        if active {
            Style::default()
                .fg(Catppuccin::MAUVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Catppuccin::SURFACE2)
        }
    }

    fn title_style(active: bool) -> Style {
        if active {
            Style::default()
                .fg(Catppuccin::LAVENDER)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Catppuccin::SUBTEXT0)
        }
    }

    fn stack_border_color() -> Color {
        Catppuccin::SURFACE2
    }

    fn stack_title_style() -> Style {
        Style::default().fg(Catppuccin::LAVENDER)
    }

    fn text_color() -> Color {
        Catppuccin::TEXT
    }

    fn muted_color() -> Color {
        Catppuccin::OVERLAY0
    }

    fn subtext_color() -> Color {
        Catppuccin::SUBTEXT0
    }

    fn badge_text_color() -> Color {
        Catppuccin::CRUST
    }

    fn header_title_style() -> Style {
        Style::default()
            .fg(Catppuccin::CRUST)
            .bg(Catppuccin::MAUVE)
            .add_modifier(Modifier::BOLD)
    }

    fn status_paused_color() -> Color {
        Catppuccin::YELLOW
    }

    fn status_running_color() -> Color {
        Catppuccin::GREEN
    }

    fn status_halted_color() -> Color {
        Catppuccin::RED
    }

    fn status_breakpoint_color() -> Color {
        Catppuccin::RED
    }

    fn rom_name_color() -> Color {
        Catppuccin::SKY
    }

    fn pc_color() -> Color {
        Catppuccin::YELLOW
    }

    fn step_count_color() -> Color {
        Catppuccin::TEAL
    }

    fn breakpoint_color() -> Color {
        Catppuccin::PEACH
    }

    fn breakpoint_marker_color() -> Color {
        Catppuccin::RED
    }

    fn pc_indicator_color() -> Color {
        Catppuccin::YELLOW
    }

    fn label_address_color() -> Color {
        Catppuccin::OVERLAY0
    }

    fn label_name_color() -> Color {
        Catppuccin::TEAL
    }

    fn instruction_address_color() -> Color {
        Catppuccin::SKY
    }

    fn instruction_bytes_color() -> Color {
        Catppuccin::OVERLAY0
    }

    fn mnemonic_color(category: MnemonicCategory) -> Color {
        match category {
            MnemonicCategory::ControlFlow => Catppuccin::MAUVE,
            MnemonicCategory::Literal => Catppuccin::PEACH,
            MnemonicCategory::Arithmetic => Catppuccin::GREEN,
            MnemonicCategory::Comparison => Catppuccin::YELLOW,
            MnemonicCategory::MemoryOrStack => Catppuccin::SAPPHIRE,
            MnemonicCategory::Raw => Catppuccin::SUBTEXT0,
        }
    }

    fn jump_target_color() -> Color {
        Catppuccin::GREEN
    }

    fn symbol_color() -> Color {
        Catppuccin::TEAL
    }

    fn literal_color() -> Color {
        Catppuccin::PEACH
    }

    fn selected_line_background_color() -> Color {
        Catppuccin::SURFACE1
    }

    fn pc_line_background_color() -> Color {
        Catppuccin::SURFACE0
    }

    fn stack_index_color() -> Color {
        Catppuccin::OVERLAY1
    }

    fn stack_byte_color() -> Color {
        Catppuccin::PEACH
    }

    fn stack_decimal_color() -> Color {
        Catppuccin::GREEN
    }

    fn stack_top_marker_color() -> Color {
        Catppuccin::MAUVE
    }

    fn footer_bracket_color() -> Color {
        Catppuccin::OVERLAY0
    }

    fn shortcut_key_color() -> Color {
        Catppuccin::MAUVE
    }

    fn shortcut_description_color() -> Color {
        Catppuccin::SUBTEXT0
    }
}
