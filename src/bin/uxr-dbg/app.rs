use std::collections::HashSet;

use uxr::uxn::vm::{Vm, VmError, VmState};
use uxr::uxntal::SymbolTable;

use super::device::CaptureDevice;
use super::disassembly_view::DisassemblyView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Paused,
    Running,
    Halted,
    Breakpoint(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Disassembly,
    Console,
}

pub struct App {
    pub vm: Vm<CaptureDevice>,
    pub rom: Vec<u8>,
    pub rom_name: String,
    pub disassembly: DisassemblyView,
    pub breakpoints: HashSet<u16>,
    pub execution_state: ExecutionState,
    pub selected_line: usize,
    pub scroll_offset: usize,
    pub active_pane: ActivePane,
    pub console_scroll: usize,
    pub instruction_count: u64,
}

impl App {
    pub fn new(
        rom: Vec<u8>,
        rom_name: String,
        symbol_table: Option<&SymbolTable>,
    ) -> Result<Self, VmError> {
        let device = CaptureDevice::new();
        let vm = Vm::new_with_rom_and_device(&rom, device)?;
        let disassembly = DisassemblyView::new(&rom, symbol_table, 0x0100);

        let mut app = Self {
            vm,
            rom,
            rom_name,
            disassembly,
            breakpoints: HashSet::new(),
            execution_state: ExecutionState::Paused,
            selected_line: 0,
            scroll_offset: 0,
            active_pane: ActivePane::Disassembly,
            console_scroll: 0,
            instruction_count: 0,
        };

        app.follow_pc();
        Ok(app)
    }

    pub fn step(&mut self) {
        if self.execution_state == ExecutionState::Halted {
            return;
        }

        match self.vm.step() {
            Ok(VmState::Ok) => {
                self.instruction_count += 1;
                self.execution_state = if self.vm.device.is_terminated() {
                    ExecutionState::Halted
                } else {
                    ExecutionState::Paused
                };
            }
            Ok(VmState::Halted) | Err(_) => {
                self.execution_state = ExecutionState::Halted;
            }
        }

        self.follow_pc();
    }

    pub fn continue_execution(&mut self) {
        if self.execution_state == ExecutionState::Halted {
            return;
        }

        // If currently stopped at a breakpoint, step one instruction first
        // so resume doesn't immediately re-trigger on the same instruction.
        if self.breakpoints.contains(&self.vm.memory.pc()) {
            self.step();
            if self.execution_state == ExecutionState::Halted {
                return;
            }
        }

        self.execution_state = ExecutionState::Running;
    }

    pub fn pause(&mut self) {
        if self.execution_state == ExecutionState::Running {
            self.execution_state = ExecutionState::Paused;
            self.follow_pc();
        }
    }

    pub fn tick(&mut self, max_instructions: usize) {
        if self.execution_state != ExecutionState::Running {
            return;
        }

        for _ in 0..max_instructions {
            let current_pc = self.vm.memory.pc();
            if self.breakpoints.contains(&current_pc) {
                self.execution_state = ExecutionState::Breakpoint(current_pc);
                self.follow_pc();
                return;
            }

            match self.vm.step() {
                Ok(VmState::Ok) => {
                    self.instruction_count += 1;
                    if self.vm.device.is_terminated() {
                        self.execution_state = ExecutionState::Halted;
                        self.follow_pc();
                        return;
                    }
                }
                Ok(VmState::Halted) | Err(_) => {
                    self.execution_state = ExecutionState::Halted;
                    self.follow_pc();
                    return;
                }
            }
        }

        self.follow_pc();
    }

    pub fn toggle_breakpoint(&mut self) {
        if let Some(address) = self.disassembly.address_for_line_index(self.selected_line) {
            if self.breakpoints.contains(&address) {
                self.breakpoints.remove(&address);
            } else {
                self.breakpoints.insert(address);
            }
        }
    }

    pub fn reset(&mut self) {
        let device = CaptureDevice::new();
        if let Ok(new_vm) = Vm::new_with_rom_and_device(&self.rom, device) {
            self.vm = new_vm;
            self.execution_state = ExecutionState::Paused;
            self.instruction_count = 0;
            self.console_scroll = 0;
            self.follow_pc();
        }
    }

    pub fn follow_pc(&mut self) {
        let pc = self.vm.memory.pc();
        if let Some(line_index) = self.disassembly.line_index_for_address(pc) {
            self.selected_line = line_index;
            self.ensure_visible(line_index);
        }
    }

    pub fn cursor_up(&mut self) {
        if self.selected_line > 0 {
            self.selected_line -= 1;
            self.ensure_visible(self.selected_line);
        }
    }

    pub fn cursor_down(&mut self) {
        if self.selected_line + 1 < self.disassembly.line_count() {
            self.selected_line += 1;
            self.ensure_visible(self.selected_line);
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_line = self.selected_line.saturating_sub(page_size);
        self.ensure_visible(self.selected_line);
    }

    pub fn page_down(&mut self, page_size: usize) {
        let max_line = self.disassembly.line_count().saturating_sub(1);
        self.selected_line = (self.selected_line + page_size).min(max_line);
        self.ensure_visible(self.selected_line);
    }

    pub fn ensure_visible(&mut self, target_line: usize) {
        // Keep selected line roughly in view with 4 lines padding
        if target_line < self.scroll_offset + 3 {
            self.scroll_offset = target_line.saturating_sub(3);
        } else if target_line >= self.scroll_offset + 20 {
            self.scroll_offset = target_line.saturating_sub(15);
        }
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Disassembly => ActivePane::Console,
            ActivePane::Console => ActivePane::Disassembly,
        };
    }

    /// Scrolls the console pane up (towards older output).
    pub fn console_scroll_up(&mut self) {
        self.console_scroll = self.console_scroll.saturating_add(1);
    }

    /// Scrolls the console pane down (towards newer output / bottom).
    pub fn console_scroll_down(&mut self) {
        self.console_scroll = self.console_scroll.saturating_sub(1);
    }

    /// Scrolls the console pane up by multiple lines (e.g. PageUp).
    pub fn console_page_up(&mut self, page_size: usize) {
        self.console_scroll = self.console_scroll.saturating_add(page_size);
    }

    /// Scrolls the console pane down by multiple lines (e.g. PageDown).
    pub fn console_page_down(&mut self, page_size: usize) {
        self.console_scroll = self.console_scroll.saturating_sub(page_size);
    }
}
