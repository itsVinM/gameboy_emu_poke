use crate::mmu::Bus;

pub struct PPU{
    pub frame: [u8; 160*144], // color indices 0-3
}

impl PPU {
    pub fn new() -> Self {
        Self{frame: [0; 160*144],}
    }

    pub fn render_scanline(&mut self, ly: u8, bus: &mut dyn Bus){
        let (lcdc, scy, scx, bgp) = (bus.read(0xFF40), bus.read(0xFF42), bus.read(0xFF43), bus.read(0xFF47));

        for x in 0..160u8 {
            let (px, py) = (x.wrapping_add(scx), ly.wrapping_add(scy));

            // Get title id from map
            let map_addr = (if lcdc & 0x08 != 0 {0x9C00} else {0x9800}) + (py as u16 / 8 * 32) + (px as u16 / 8);
            let tid = bus.read(map_addr);

            // Get pixel data from tile
            let addr = if lcdc & 0x10 != 0 { 0x8000 + (tid as u16 * 16)}
                        else {0x9000u16.wrapping_add(((tid as i8 as i16) * 16) as u16) } + (py as u16 % 8 * 2);
            let (b1, b2, bit) = (bus.read(addr), bus.read(addr + 1), 7 - (px % 8));
            let color_idx = ((b2 >> bit) & 1) << 1 | ((b1 >> bit) & 1);

            // Store PPU's frame buffer
            self.frame[ly as usize * 160 + x as usize] = (bgp >> (color_idx * 2)) & 0x03;
        }
    }
    
}