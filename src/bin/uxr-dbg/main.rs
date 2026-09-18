mod app;
mod device;
mod disassembly_view;
mod theme;
mod theme_catpuccin;
mod ui;

/// Active theme for the debugger user interface.
pub type Theme = theme_catpuccin::CatppuccinTheme;

use std::fs;
use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::ExecutableCommand;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use uxr::uxntal::SymbolTable;

use app::{ActivePane, App, ExecutionState};

// Poll for events at ~60Hz
const POLL_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Parser, Debug)]
#[command(
    name = "uxr-dbg",
    about = "Interactive terminal debugger for the Uxn virtual machine"
)]
struct Args {
    /// Path to the UXN ROM file (.rom)
    rom_path: PathBuf,

    /// Optional path to the symbol file (.sym). Defaults to <rom-file>.sym if present.
    #[arg(long)]
    symbol_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let arguments = Args::parse();

    let rom_data = fs::read(&arguments.rom_path)
        .with_context(|| format!("Failed to read ROM file: {}", arguments.rom_path.display()))?;

    let symbol_file_path =
        resolve_symbol_path(&arguments.rom_path, arguments.symbol_path.as_deref());
    let symbol_table = symbol_file_path.and_then(|path| SymbolTable::from_file(&path).ok());

    let rom_filename = arguments
        .rom_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "ROM".to_string());

    // Setup terminal and panic hook
    setup_panic_hook();
    enable_raw_mode().context("Failed to enable terminal raw mode")?;
    stdout()
        .execute(EnterAlternateScreen)
        .context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal")?;

    let mut app = App::new(rom_data, rom_filename, symbol_table.as_ref())
        .map_err(|error| anyhow::anyhow!("Failed to initialize VM: {error}"))?;

    let run_result = run(&mut terminal, &mut app);

    // Restore terminal on exit
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);
    let _ = stdout().execute(Show);

    run_result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if app.execution_state == ExecutionState::Running {
            app.tick(1000);
        }

        if !event::poll(POLL_INTERVAL)? {
            continue;
        }

        let event = event::read()?;
        if let Event::Key(key_event) = event
            && matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && !handle_key_events(key_event, app)
        {
            break;
        }
    }

    Ok(())
}

fn handle_key_events(key_event: KeyEvent, app: &mut App) -> bool {
    // Check for Ctrl+C to force exit
    if key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c') {
        return false;
    }

    match key_event.code {
        KeyCode::Char('q') => return false,
        KeyCode::Char('s') | KeyCode::Char(' ') | KeyCode::F(7) => {
            if app.execution_state == ExecutionState::Running {
                app.pause();
            } else {
                app.step();
            }
        }
        KeyCode::Char('c') | KeyCode::F(5) => {
            app.continue_execution();
        }
        KeyCode::Char('p') => {
            app.pause();
        }
        KeyCode::Char('b') | KeyCode::F(9) => {
            app.toggle_breakpoint();
        }
        KeyCode::Char('r') => {
            app.reset();
        }
        KeyCode::Char('g') => {
            app.follow_pc();
        }
        KeyCode::Tab => {
            app.toggle_pane();
        }
        KeyCode::Up | KeyCode::Char('k') => match app.active_pane {
            ActivePane::Disassembly => app.cursor_up(),
            ActivePane::Console => app.console_scroll_up(),
        },
        KeyCode::Down | KeyCode::Char('j') => match app.active_pane {
            ActivePane::Disassembly => app.cursor_down(),
            ActivePane::Console => app.console_scroll_down(),
        },
        KeyCode::PageUp => match app.active_pane {
            ActivePane::Disassembly => app.page_up(15),
            ActivePane::Console => app.console_page_up(10),
        },
        KeyCode::PageDown => match app.active_pane {
            ActivePane::Disassembly => app.page_down(15),
            ActivePane::Console => app.console_page_down(10),
        },
        _ => {}
    }

    true
}

fn resolve_symbol_path(rom_path: &Path, explicit_sym: Option<&Path>) -> Option<PathBuf> {
    if let Some(explicit) = explicit_sym.filter(|path| path.exists()) {
        return Some(explicit.to_path_buf());
    }

    // Check <rom_path>.sym (e.g. hello.rom.sym)
    let candidate_dot_sym = PathBuf::from(format!("{}.sym", rom_path.display()));
    if candidate_dot_sym.exists() {
        return Some(candidate_dot_sym);
    }

    // Check with extension replaced (e.g. hello.sym)
    let candidate_ext = rom_path.with_extension("sym");
    if candidate_ext.exists() {
        return Some(candidate_ext);
    }

    None
}

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = stdout().execute(Show);
        original_hook(panic_info);
    }));
}
