use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use uxr::uxntal::InstructionKind;

use super::Theme;
use super::app::{ActivePane, App, ExecutionState};
use super::disassembly_view::DisassemblyLineKind;
use super::theme::ThemeTrait;

/// Renders the complete debugger terminal user interface.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Top-level layout: Header, Main Area, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let pc = app.vm.memory.pc();

    let status_color = Theme::status_color(app.execution_state);
    let status_text = match app.execution_state {
        ExecutionState::Paused => " PAUSED ".to_string(),
        ExecutionState::Running => " RUNNING ".to_string(),
        ExecutionState::Halted => " HALTED ".to_string(),
        ExecutionState::Breakpoint(breakpoint) => format!(" BREAKPOINT 0x{breakpoint:04x} "),
    };

    let header_line = Line::from(vec![
        Span::styled(" uxr-dbg ", Theme::header_title_style()),
        Span::raw("  "),
        Span::styled(status_text, Theme::status_badge_style(status_color)),
        Span::raw("  ROM: "),
        Span::styled(
            &app.rom_name,
            Style::default()
                .fg(Theme::rom_name_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} bytes)", app.rom.len()),
            Style::default().fg(Theme::muted_color()),
        ),
        Span::raw("   PC: "),
        Span::styled(
            format!("0x{pc:04x}"),
            Style::default()
                .fg(Theme::pc_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   Steps: "),
        Span::styled(
            format!("{}", app.instruction_count),
            Style::default().fg(Theme::step_count_color()),
        ),
        Span::raw("   Breakpoints: "),
        Span::styled(
            format!("{}", app.breakpoints.len()),
            Style::default().fg(Theme::breakpoint_color()),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::bar_border_color()))
        .style(Style::default().bg(Theme::bar_background_color()));

    let paragraph = Paragraph::new(header_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_disassembly(frame, app, main_chunks[0]);
    render_right_column(frame, app, main_chunks[1]);
}

fn render_disassembly(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_pane == ActivePane::Disassembly;
    let pc = app.vm.memory.pc();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_style(is_active))
        .title(" Disassembly ")
        .title_style(Theme::title_style(is_active))
        .style(Style::default().bg(Theme::background_color()));

    let inner_area = block.inner(area);
    let visible_lines = inner_area.height as usize;

    let total_lines = app.disassembly.line_count();
    let start_line = app.scroll_offset.min(total_lines);
    let end_line = (start_line + visible_lines).min(total_lines);

    let mut rendered_lines = Vec::with_capacity(end_line.saturating_sub(start_line));

    for line_index in start_line..end_line {
        let is_selected = line_index == app.selected_line;
        let line_data = &app.disassembly.lines()[line_index];

        let line_address = line_data.address;
        let is_pc = line_address == Some(pc)
            && matches!(line_data.kind, DisassemblyLineKind::Instruction { .. });
        let has_breakpoint = line_address
            .map(|address| app.breakpoints.contains(&address))
            .unwrap_or(false);

        let mut spans = Vec::new();

        // Breakpoint marker column
        if has_breakpoint {
            spans.push(Span::styled(
                "● ",
                Style::default()
                    .fg(Theme::breakpoint_marker_color())
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("  "));
        }

        // PC Indicator column
        if is_pc {
            spans.push(Span::styled(
                "-> ",
                Style::default()
                    .fg(Theme::pc_indicator_color())
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("   "));
        }

        // Instruction or Label content
        match &line_data.kind {
            DisassemblyLineKind::Label { name, address } => {
                spans.push(Span::styled(
                    format!("{address:04x} "),
                    Style::default().fg(Theme::label_address_color()),
                ));
                spans.push(Span::styled(
                    format!("<{name}>:"),
                    Style::default()
                        .fg(Theme::label_name_color())
                        .add_modifier(Modifier::BOLD),
                ));
            }
            DisassemblyLineKind::Instruction {
                instruction,
                target_symbol,
            } => {
                // Address: e.g. "0100: "
                spans.push(Span::styled(
                    format!("{:04x}: ", instruction.address),
                    Style::default().fg(Theme::instruction_address_color()),
                ));

                // Raw bytes: e.g. "80 12     "
                let mut bytes_str = String::new();
                for (byte_index, byte) in instruction.bytes.iter().enumerate() {
                    if byte_index > 0 {
                        bytes_str.push(' ');
                    }
                    bytes_str.push_str(&format!("{byte:02x}"));
                }
                spans.push(Span::styled(
                    format!("{bytes_str:<9} "),
                    Style::default().fg(Theme::instruction_bytes_color()),
                ));

                // Mnemonic: e.g. "LIT2    "
                let mnemonic_color = Theme::mnemonic_color(instruction.category);
                spans.push(Span::styled(
                    format!("{:<7} ", instruction.mnemonic),
                    Style::default()
                        .fg(mnemonic_color)
                        .add_modifier(Modifier::BOLD),
                ));

                // Operands & Symbols
                match &instruction.kind {
                    InstructionKind::Jci { target_address, .. }
                    | InstructionKind::Jmi { target_address, .. }
                    | InstructionKind::Jsi { target_address, .. } => {
                        spans.push(Span::styled(
                            format!("{target_address:04x} "),
                            Style::default().fg(Theme::jump_target_color()),
                        ));
                        if let Some(target) = target_symbol {
                            spans.push(Span::styled(
                                format!("<{target}>"),
                                Style::default().fg(Theme::symbol_color()),
                            ));
                        }
                    }
                    InstructionKind::Lit { value } => {
                        spans.push(Span::styled(
                            format!("#{value:02x}"),
                            Style::default().fg(Theme::literal_color()),
                        ));
                        if value.is_ascii_graphic() {
                            spans.push(Span::styled(
                                format!(" '{}'", *value as char),
                                Style::default().fg(Theme::subtext_color()),
                            ));
                        }
                    }
                    InstructionKind::Lit2 { value } => {
                        spans.push(Span::styled(
                            format!("#{value:04x}"),
                            Style::default().fg(Theme::literal_color()),
                        ));
                        if let Some(target) = target_symbol {
                            spans.push(Span::styled(
                                format!(" <{target}>"),
                                Style::default().fg(Theme::symbol_color()),
                            ));
                        }
                    }
                    InstructionKind::Litr { value } => {
                        spans.push(Span::styled(
                            format!("#{value:02x}"),
                            Style::default().fg(Theme::literal_color()),
                        ));
                    }
                    InstructionKind::Lit2r { value } => {
                        spans.push(Span::styled(
                            format!("#{value:04x}"),
                            Style::default().fg(Theme::literal_color()),
                        ));
                    }
                    InstructionKind::RawByte(byte) => {
                        spans.push(Span::styled(
                            format!("{byte:02x}"),
                            Style::default().fg(Theme::subtext_color()),
                        ));
                    }
                    InstructionKind::Brk | InstructionKind::Regular { .. } => {}
                }
            }
            DisassemblyLineKind::OmittedZeroes => {
                spans.push(Span::styled(
                    "         ...",
                    Style::default()
                        .fg(Theme::muted_color())
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }

        let mut line = Line::from(spans);
        if is_selected {
            line = line.style(Style::default().bg(Theme::selected_line_background_color()));
        } else if is_pc {
            line = line.style(Style::default().bg(Theme::pc_line_background_color()));
        }

        rendered_lines.push(line);
    }

    let paragraph = Paragraph::new(rendered_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_right_column(frame: &mut Frame, app: &mut App, area: Rect) {
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(28),
            Constraint::Percentage(38),
        ])
        .split(area);

    render_stack(
        frame,
        " Working Stack (wst) ",
        app.vm.ws.pointer(),
        app.vm.ws.as_slice(),
        right_chunks[0],
    );
    render_stack(
        frame,
        " Return Stack (rst) ",
        app.vm.rs.pointer(),
        app.vm.rs.as_slice(),
        right_chunks[1],
    );
    render_console(frame, app, right_chunks[2]);
}

fn render_stack(frame: &mut Frame, title: &str, pointer: u8, slice: &[u8], area: Rect) {
    let title_with_count = format!("{title}[sp: 0x{pointer:02x}] ");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::stack_border_color()))
        .title(title_with_count)
        .title_style(Theme::stack_title_style())
        .style(Style::default().bg(Theme::background_color()));

    let inner_height = block.inner(area).height as usize;
    let mut lines = Vec::new();

    if slice.is_empty() {
        lines.push(Line::from(Span::styled(
            "  <empty>",
            Style::default().fg(Theme::muted_color()),
        )));
    } else {
        // Display items from top of stack down to bottom
        let total_items = slice.len();
        let items_to_show = total_items.min(inner_height);
        let start_index = total_items.saturating_sub(items_to_show);

        for (display_index, i) in (start_index..total_items).rev().enumerate() {
            let byte = slice[i];
            let is_top = display_index == 0;

            let char_repr = if byte.is_ascii_graphic() {
                format!("'{}'", byte as char)
            } else if byte == b' ' {
                "' '".to_string()
            } else {
                "   ".to_string()
            };

            let mut spans = vec![
                Span::styled(
                    format!("  [{i:02x}] "),
                    Style::default().fg(Theme::stack_index_color()),
                ),
                Span::styled(
                    format!("0x{byte:02x} "),
                    Style::default()
                        .fg(Theme::stack_byte_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({byte:>3}) "),
                    Style::default().fg(Theme::stack_decimal_color()),
                ),
                Span::styled(
                    format!("{char_repr:<4}"),
                    Style::default().fg(Theme::text_color()),
                ),
            ];

            if is_top {
                spans.push(Span::styled(
                    " <-- top",
                    Style::default()
                        .fg(Theme::stack_top_marker_color())
                        .add_modifier(Modifier::BOLD),
                ));
            }

            lines.push(Line::from(spans));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_console(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_pane == ActivePane::Console;

    let inner_height = area.height.saturating_sub(2) as usize;
    let console_lines = app.vm.device.console_lines();
    let total_lines = console_lines.len();

    let is_empty =
        console_lines.is_empty() || (console_lines.len() == 1 && console_lines[0].is_empty());

    let max_scroll = if is_empty {
        0
    } else {
        total_lines.saturating_sub(inner_height)
    };
    app.console_scroll = app.console_scroll.min(max_scroll);

    let title = if app.console_scroll > 0 {
        format!(" Console Output [-{}] ", app.console_scroll)
    } else {
        " Console Output ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_style(is_active))
        .title(title)
        .title_style(Theme::title_style(is_active))
        .style(Style::default().bg(Theme::background_color()));

    let lines = if is_empty {
        vec![Line::from(Span::styled(
            "  <no output>",
            Style::default().fg(Theme::muted_color()),
        ))]
    } else {
        let start_line = max_scroll.saturating_sub(app.console_scroll);

        console_lines
            .iter()
            .skip(start_line)
            .take(inner_height)
            .map(|line| Line::from(Span::styled(line, Style::default().fg(Theme::text_color()))))
            .collect()
    };

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, _app: &App, area: Rect) {
    let shortcuts = [
        ("s/Space", "Step"),
        ("c", "Continue"),
        ("p", "Pause"),
        ("b", "Breakpoint"),
        ("r", "Reset"),
        ("g", "Go to PC"),
        ("j/k", "Scroll"),
        ("Tab", "Switch Pane"),
        ("q", "Quit"),
    ];

    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    for (key, desc) in shortcuts {
        spans.push(Span::styled(
            "[",
            Style::default().fg(Theme::footer_bracket_color()),
        ));
        spans.push(Span::styled(
            key,
            Style::default()
                .fg(Theme::shortcut_key_color())
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "] ",
            Style::default().fg(Theme::footer_bracket_color()),
        ));
        spans.push(Span::styled(
            desc,
            Style::default().fg(Theme::shortcut_description_color()),
        ));
        spans.push(Span::raw("  "));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::bar_border_color()))
        .style(Style::default().bg(Theme::bar_background_color()));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(paragraph, area);
}
