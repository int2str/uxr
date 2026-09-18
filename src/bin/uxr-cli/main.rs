use anyhow::Result;

use uxr::shared::file_io::read_file_or_stdin;
use uxr::uxn::vm::Vm;

mod console_device;
use console_device::ConsoleDevice;

fn main() -> Result<()> {
    let buffer = read_file_or_stdin()?;

    let mut vm = Vm::new_with_rom_and_device(&buffer, ConsoleDevice::new())?;
    vm.run().map_err(Into::into)
}
