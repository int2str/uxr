use crate::uxn::memory::Memory;

/// Defines the interface for devices attached to the UXN VM.
pub trait VmDevice {
    /// Reads a byte from the specified device port.
    fn input(&mut self, port: u8) -> u8;

    /// Writes a byte to the specified device port.
    fn output(&mut self, port: u8, value: u8, memory: &mut Memory);
}

/// A no-op device that ignores all output and returns 0 for all input.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NullDevice;

impl VmDevice for NullDevice {
    fn input(&mut self, _port: u8) -> u8 {
        0
    }

    fn output(&mut self, _port: u8, _value: u8, _memory: &mut Memory) {}
}

impl<D: VmDevice + ?Sized> VmDevice for Box<D> {
    fn input(&mut self, port: u8) -> u8 {
        (**self).input(port)
    }

    fn output(&mut self, port: u8, value: u8, memory: &mut Memory) {
        (**self).output(port, value, memory);
    }
}

impl<D: VmDevice + ?Sized> VmDevice for &mut D {
    fn input(&mut self, port: u8) -> u8 {
        (**self).input(port)
    }

    fn output(&mut self, port: u8, value: u8, memory: &mut Memory) {
        (**self).output(port, value, memory);
    }
}
