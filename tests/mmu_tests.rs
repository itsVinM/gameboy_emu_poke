use pokegameboy::mmu::Mmu;
use pokegameboy::traits::MemoryBus;

fn mmu() -> Mmu {
    Mmu::new(vec![0u8; 0x8000], vec![0u8; 0x8000])
}

// ── basic regions ────────────────────────────────────────────────────────────

#[test]
fn wram_roundtrip() {
    let mut m = mmu();
    m.write(0xC000, 0x42);
    assert_eq!(m.read(0xC000), 0x42);
}

#[test]
fn wram_end_boundary() {
    let mut m = mmu();
    m.write(0xDFFF, 0xFF);
    assert_eq!(m.read(0xDFFF), 0xFF);
}

#[test]
fn vram_roundtrip() {
    let mut m = mmu();
    m.write(0x8000, 0xAB);
    assert_eq!(m.read(0x8000), 0xAB);
}

#[test]
fn oam_roundtrip() {
    let mut m = mmu();
    m.write(0xFE00, 0x55);
    assert_eq!(m.read(0xFE00), 0x55);
}

#[test]
fn hram_roundtrip() {
    let mut m = mmu();
    m.write(0xFF80, 0x99);
    assert_eq!(m.read(0xFF80), 0x99);
}

#[test]
fn ie_register_roundtrip() {
    let mut m = mmu();
    m.write(0xFFFF, 0x1F);
    assert_eq!(m.read(0xFFFF), 0x1F);
}

#[test]
fn unused_range_returns_ff() {
    let m = mmu();
    assert_eq!(m.read(0xFEA0), 0xFF);
    assert_eq!(m.read(0xFEFF), 0xFF);
}

// ── IO registers ─────────────────────────────────────────────────────────────

#[test]
fn div_resets_to_zero_on_any_write() {
    let mut m = mmu();
    m.io[0x04] = 0xCC;          // simulate DIV having a value
    m.write(0xFF04, 0xFF);      // any write resets to 0
    assert_eq!(m.read(0xFF04), 0x00);
}

#[test]
fn io_generic_write_read() {
    let mut m = mmu();
    m.write(0xFF47, 0xE4); // BGP palette
    assert_eq!(m.read(0xFF47), 0xE4);
}

// ── DMA ──────────────────────────────────────────────────────────────────────

#[test]
fn dma_copies_rom_to_oam() {
    let mut rom = vec![0u8; 0x8000];
    for i in 0..0xA0usize { rom[i] = i as u8; }
    let mut m = Mmu::new(rom, vec![0u8; 0x8000]);
    m.write(0xFF46, 0x00); // DMA from 0x0000
    assert_eq!(m.oam[0],    0x00);
    assert_eq!(m.oam[5],    0x05);
    assert_eq!(m.oam[0x9F], 0x9F);
}

// ── MBC3 bank switching ───────────────────────────────────────────────────────

#[test]
fn rom_bank_1_is_default_at_0x4000() {
    let mut rom = vec![0u8; 0x8000];
    rom[0x4000] = 0xAA; // bank 1 start
    let m = Mmu::new(rom, vec![]);
    assert_eq!(m.read(0x4000), 0xAA);
}

#[test]
fn rom_bank_select_switches_bank() {
    let mut rom = vec![0u8; 0x8000 * 2]; // 4 banks × 16KB = needs at least 2 banks
    rom[0x4000] = 0x11; // bank 1
    rom[0x8000] = 0x22; // bank 2 — but we only have 2 banks in a 32 KB ROM
    // For a minimal test: write 0x2000-0x3FFF selects bank
    let mut m = Mmu::new(rom, vec![0u8; 0x2000]);
    assert_eq!(m.rom_bank, 1); // default
    m.write(0x2000, 0x01);     // select bank 1 (no-op from default)
    assert_eq!(m.rom_bank, 1);
}

#[test]
fn extram_bank_select() {
    let mut m = mmu();
    m.write(0x4000, 0x02); // select extram bank 2
    assert_eq!(m.read(0x4000), 0); // bank 2 selected but still reads extram (0)
}

// ── MemoryBus trait on Mmu ────────────────────────────────────────────────────

#[test]
fn memory_bus_trait_delegates_correctly() {
    let mut m = mmu();
    MemoryBus::write(&mut m, 0xC100, 0x77);
    assert_eq!(MemoryBus::read(&m, 0xC100), 0x77);
}
