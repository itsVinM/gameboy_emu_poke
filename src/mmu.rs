pub trait MemoryBus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub trait Tickable {
    fn tick<B: MemoryBus>(&mut self, cycles: u32, bus: &mut B);
}

pub struct Mmu {
    rom:         Vec<u8>,
    rom_bank:    usize,
    vram:        [u8; 0x2000],
    extram:      Vec<u8>,
    extram_bank: usize,
    wram:        [u8; 0x4000],
    oam:         [u8; 0xA0],
    pub io:      [u8; 0x80],
    hram:        [u8; 0x7F],
    ie:          u8,
    pub buttons:   u8,
    pub dpad:      u8,
    pub prev_joyp: u8,
}

impl Mmu {
    pub fn new(rom: Vec<u8>, extram: Vec<u8>) -> Self {
        let mut mmu = Self {
            rom, rom_bank: 1,
            vram: [0; 0x2000], extram, extram_bank: 0,
            wram: [0; 0x4000], oam: [0; 0xA0],
            io: [0; 0x80], hram: [0; 0x7F], ie: 0,
            buttons: 0x0F, dpad: 0x0F, prev_joyp: 0x0F,
        };
        mmu.io[0x40] = 0x91; // LCDC
        mmu.io[0x47] = 0xFC; // BGP
        mmu
    }

    pub fn get_save_data(&self) -> Vec<u8> { self.extram.clone() }

    pub fn load_save_data(&mut self, data: Vec<u8>) {
        let n = data.len().min(self.extram.len());
        self.extram[..n].copy_from_slice(&data[..n]);
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => self.rom[self.rom_bank * 0x4000 + (addr as usize - 0x4000)],
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000],
            0xA000..=0xBFFF => *self.extram.get(self.extram_bank * 0x2000 + (addr as usize - 0xA000)).unwrap_or(&0xFF),
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000],
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000],
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00..=0xFF7F => {
                if addr == 0xFF00 {
                    let sel = self.io[0x00] & 0x30;
                    let nibble = if sel & 0x10 == 0 { self.dpad }
                                 else if sel & 0x20 == 0 { self.buttons }
                                 else { 0x0F };
                    sel | 0xC0 | (nibble & 0x0F)
                } else {
                    self.io[addr as usize - 0xFF00]
                }
            }
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80],
            0xFFFF          => self.ie,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {}
            0x2000..=0x3FFF => self.rom_bank = if val == 0 { 1 } else { (val & 0x7F) as usize },
            0x4000..=0x5FFF => { if val <= 3 { self.extram_bank = val as usize; } }
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = val,
            0xA000..=0xBFFF => {
                let off = self.extram_bank * 0x2000 + (addr as usize - 0xA000);
                if off < self.extram.len() { self.extram[off] = val; }
            }
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000] = val,
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000] = val,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = val,
            0xFF00..=0xFF7F => {
                let i = addr as usize - 0xFF00;
                match addr {
                    0xFF46 => { // DMA
                        let src = (val as u16) << 8;
                        for j in 0..0xA0u16 { self.oam[j as usize] = self.read(src + j); }
                    }
                    0xFF04 => self.io[i] = 0, // DIV resets on any write
                    _      => self.io[i] = val,
                }
            }
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80] = val,
            0xFFFF          => self.ie = val,
            _               => {}
        }
    }
}

impl MemoryBus for Mmu {
    #[inline(always)] fn read(&self, addr: u16) -> u8 { Mmu::read(self, addr) }
    #[inline(always)] fn write(&mut self, addr: u16, val: u8) { Mmu::write(self, addr, val) }
}
