// src/mmu.rs

pub trait Bus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub struct MainBus{
    pub rom: Vec<u8>,       // Heap allocated (game)
    pub joypad_state: u8,   // 0xFF00
    pub vram: [u8; 8192],   // Stack/static allocated - graphics
    pub wram: [u8; 8192],   // work RAM
    pub oam:  [u8; 160],
    pub hram: [u8; 128],    // "high RAM" - internal to CPU
    pub io:   [u8; 128],    // hw registers - buttons/sound
    pub ie_reg: u8,
}

impl MainBus {
    pub fn new(rom_data: Vec<u8>) -> Self{
        
        Self{
            rom:  rom_data,
            joypad_state: 0xFF,
            vram: [0; 8192],
            wram: [0; 8192],
            oam:  [0; 160],
            hram: [0; 128],
            io:   [0; 128],
            ie_reg: 0,
        }
    } 
}

impl Bus for MainBus{
    
    fn read(&self, addr: u16) -> u8{

        match addr {
            0x0000..=0x7FFF => self.rom[addr as usize],
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            
            // Combine WRAM and Echo RAM into one logic block
            // 0xC000 & 0x1FFF = 0x0000
            // 0xE000 & 0x1FFF = 0x0000
            0xC000..=0xFDFF => self.wram[(addr & 0x1FFF) as usize],
            
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFF00          => self.joypad_state,
            0xFF01..=0xFF7F => self.io[(addr - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF          => self.ie_reg,
            _               => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, val: u8){
        match addr {
            0x0000..=0x7FFF => {},
            // VRAM
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = val,

            // Work RAM & Echo RAM (0xC000 - 0xFDFF)
            // Using & 0x1FFF maps both ranges to 0..8191
            0xC000..=0xFDFF => self.wram[(addr & 0x1FFF) as usize] = val,

            // OAM
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = val,

            // Joypad
            0xFF00 => {
                let current = self.io[0];
                self.io[0] = (val & 0x30) | (current & 0xCF);
            },

            // DMA Transfer
            0xFF46 => {
                self.io[0x46] = val; // Store the source high-byte
                let source_base = (val as u16) << 8;
                for i in 0..160 {
                    // We use self.read to ensure we handle ROM/WRAM/Echo sources correctly
                    let byte = self.read(source_base + i);
                    self.oam[i as usize] = byte;
                }
            },

            // Other I/O
            0xFF01..=0xFF7F => self.io[(addr - 0xFF00) as usize] = val,

            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = val,

            // Interrupt Enable
            0xFFFF => self.ie_reg = val,

    
            _ => {} // ROM read-only
        }
    }


}
