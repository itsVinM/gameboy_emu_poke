pub struct Mmu {
    rom:          Vec<u8>,
    rom_bank:     usize,
    pub vram:     [u8; 0x2000],
    pub extram:   Vec<u8>,
    extram_bank:  usize,
    wram:         [u8; 0x4000],
    pub oam:      [u8; 0xA0],
    pub io:       [u8; 0x80],
    hram:         [u8; 0x7F],
    pub ie:       u8,
    pub buttons: u8, // face buttons: Start | Select | B | A (active-low, 0=pressed)
    pub dpad: u8,   // directions: Down | Up | Left | Right 
}

impl Mmu {
    pub fn new(rom: Vec<u8>, extram: Vec<u8>) -> Self {
        let mut mmu = Self {
            rom,
            rom_bank:    1,
            vram:        [0; 0x2000],
            extram,
            extram_bank: 0,
            wram:        [0; 0x4000],
            oam:         [0; 0xA0],
            io:          [0; 0x80],
            hram:        [0; 0x7F],
            ie:          0,
            buttons: 0x0F,
            dpad: 0x0F,     // nothing pressed
        };
        // Boot state
        mmu.io[0x40] = 0x91; // LCDC
        mmu.io[0x47] = 0xFC; // BGP
        mmu
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => {
                let offset = self.rom_bank * 0x4000 + (addr as usize - 0x4000);
                *self.rom.get(offset).unwrap_or(&0xFF)
            }
            0xFF00 => {
                let select = self.io[0];
                if select & 0x10 == 0 {       // bit 4 low -> game wants D-pad
                    0xC0 | 0x10 | self.dpad  // bit 7-5 always 1, but 4 =0
                } else if select & 0x20 == 0 {//bit 5 low -> game wants buttons
                    0xC0 | 0x20 | self.buttons
                } else {
                    0xFF // nothing selected
                }
            }
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000],
            0xA000..=0xBFFF => {
                let offset = self.extram_bank * 0x2000 + (addr as usize - 0xA000);
                *self.extram.get(offset).unwrap_or(&0xFF)
            }
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000],
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000],
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00..=0xFF7F => self.io_read(addr),
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80],
            0xFFFF          => self.ie,
            
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // MBC3 ROM bank select
            0x2000..=0x3FFF => {
                self.rom_bank = if val == 0 { 1 } else { (val & 0x7F) as usize };
            }
            // MBC3 RAM bank select
            0x4000..=0x5FFF => {
                if val <= 3 { self.extram_bank = val as usize; }
            }
            0x8000..=0x9FFF => self.vram[addr as usize - 0x8000] = val,
            0xA000..=0xBFFF => {
                let offset = self.extram_bank * 0x2000 + (addr as usize - 0xA000);
                if offset < self.extram.len() {
                    self.extram[offset] = val;
                }
            }
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000] = val,
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000] = val,
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00] = val,
            0xFF00..=0xFF7F => self.io_write(addr, val),
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80] = val,
            0xFFFF          => self.ie = val,
            _               => {}
        }
    }

    fn io_read(&self, addr: u16) -> u8 {
        self.io[addr as usize - 0xFF00]
    }

    fn io_write(&mut self, addr: u16, val: u8) {
        let i = addr as usize - 0xFF00;
        match addr {
            // DMA transfer to OAM
            0xFF46 => {
                let src = (val as u16) << 8;
                for j in 0..0xA0u16 {
                    self.oam[j as usize] = self.read(src + j);
                }
            }
            // DIV resets to 0 on any write
            0xFF04 => self.io[i] = 0,
            _      => self.io[i] = val,
        }
    }
}