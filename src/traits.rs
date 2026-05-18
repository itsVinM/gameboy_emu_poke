/// Abstraction over the memory bus — any component that maps 16-bit addresses.
/// Mirrors the `Device` trait used in the RISC-V VM.
pub trait MemoryBus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

/// A component that advances by a number of CPU T-cycles.
pub trait Tickable {
    fn tick<B: MemoryBus>(&mut self, cycles: u32, bus: &mut B);
}
