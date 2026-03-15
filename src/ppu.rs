use crate::mmu::Mmu;

pub struct Ppu {
    pub framebuffer: [u8; 160 * 144 * 4],
    pub dot: u32,
    pub ly:  u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self { framebuffer: [0xFF; 160 * 144 * 4], dot: 0, ly: 0 }
    }

    pub fn tick(&mut self, cycles: u32, mmu: &mut Mmu) {
        let lcdc = mmu.io[0x40];
        if lcdc & 0x80 == 0 { 
            self.ly = 0; 
            self.dot = 0; 
            mmu.io[0x44] = 0;
            // Reset STAT to Mode 0 when LCD is off
            mmu.io[0x41] = mmu.io[0x41] & 0xFC; 
            return; 
        }

        self.dot += cycles;

        // A scanline takes exactly 456 dots
        if self.dot >= 456 {
            self.dot -= 456;
            self.ly = (self.ly + 1) % 154;
            mmu.io[0x44] = self.ly;

            // LYC Check: Bit 2 of STAT is set if LY == LYC
            if self.ly == mmu.io[0x45] {
                mmu.io[0x41] |= 0x04;
                if mmu.io[0x41] & 0x40 != 0 { mmu.io[0x0F] |= 0x02; } // STAT IRQ
            } else {
                mmu.io[0x41] &= !0x04;
            }

            if self.ly == 144 {
                mmu.io[0x0F] |= 0x01; // Request V-Blank Interrupt
            }
        }

        // --- MODE SWITCHING (The Oak Fix) ---
        let mut stat = mmu.io[0x41];
        let old_mode = stat & 0x03;
        let new_mode = if self.ly >= 144 {
            1 // Mode 1: V-Blank
        } else if self.dot < 80 {
            2 // Mode 2: OAM Search
        } else if self.dot < 252 {
            3 // Mode 3: Data Transfer
        } else {
            0 // Mode 0: H-Blank
        };

        if old_mode != new_mode {
            stat = (stat & 0xFC) | new_mode;
            
            // Mode Interrupts: Many games wait for these to progress
            let interrupt = match new_mode {
                0 => stat & 0x08 != 0, // H-Blank IRQ
                1 => stat & 0x10 != 0, // V-Blank IRQ
                2 => stat & 0x20 != 0, // OAM IRQ
                _ => false,
            };
            if interrupt { mmu.io[0x0F] |= 0x02; } // Trigger STAT Interrupt
            
            // Render exactly once per line (transition to H-Blank)
            if new_mode == 0 && self.ly < 144 {
                self.render_scanline(mmu, lcdc);
            }
        }
        mmu.io[0x41] = stat;
    }

    fn render_scanline(&mut self, mmu: &Mmu, lcdc: u8) {
        let (scx, scy) = (mmu.io[0x43], mmu.io[0x42]);
        let (wx, wy) = (mmu.io[0x4B].wrapping_sub(7), mmu.io[0x4A]);
        let bgp = mmu.io[0x47];

        (0u8..160).for_each(|x| {
            // Flattened logic: use window if enabled and within bounds, else background
            let (win, px, py) = if (lcdc & 0x20 != 0) && self.ly >= wy && x >= wx {
                (true, x - wx, self.ly - wy)
            } else {
                (false, x.wrapping_add(scx), self.ly.wrapping_add(scy))
            };

            let color = self.get_bg_pixel(mmu, lcdc, px as u16, py as u16, bgp, win);
            self.set_pixel(x as usize, self.ly as usize, color);
        });

        if lcdc & 0x02 != 0 { self.render_sprites(mmu); }
    }

    fn get_bg_pixel(&self, mmu: &Mmu, lcdc: u8, px: u16, py: u16, palette: u8, is_win: bool) -> u8 {
        let map_bit = if is_win { 0x40 } else { 0x08 };
        let map_base = if lcdc & map_bit != 0 { 0x9C00 } else { 0x9800 };
        let tile_idx = mmu.read(map_base + (py / 8) * 32 + (px / 8));
        
        let tile_addr = if lcdc & 0x10 != 0 {
            0x8000 + (tile_idx as u16 * 16)
        } else {
            (0x9000i32 + (tile_idx as i8 as i32 * 16)) as u16
        };

        let row = tile_addr + (py % 8) * 2;
        let (lo, hi) = (mmu.read(row), mmu.read(row + 1));
        let bit = 7 - (px % 8);
        let id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
        (palette >> (id * 2)) & 0x03
    }

   fn render_sprites(&mut self, mmu: &Mmu) {
        let current_ly = self.ly; // 1. Capture LY value to break the borrow chain
        let obp0 = mmu.io[0x48];
        let obp1 = mmu.io[0x49];

        mmu.oam.chunks_exact(4)
            .take(40)
            .filter_map(|s| {
                let sy = s[0] as i16 - 16;
                if (current_ly as i16) < sy || (current_ly as i16) >= sy + 8 { return None; }
                Some((s, sy))
            })
            .take(10) 
            .for_each(|(s, sy)| {
                let (sx, tile, attr) = (s[1] as i16 - 8, s[2], s[3]);
                let pal = if attr & 0x10 != 0 { obp1 } else { obp0 };
                let mut row = (current_ly as i16 - sy) as u16;
                if attr & 0x40 != 0 { row = 7 - row; }

                let addr = 0x8000 + (tile as u16 * 16) + (row * 2);
                let (lo, hi) = (mmu.read(addr), mmu.read(addr + 1));

                (0..8i16).for_each(|px| {
                    let tx = sx + px;
                    if tx < 0 || tx >= 160 { return; }
                    let bit = if attr & 0x20 != 0 { px } else { 7 - px } as u8;
                    let id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                    
                    if id == 0 { return; } 

                    // Priority check
                    if attr & 0x80 != 0 {
                        let i = (current_ly as usize * 160 + tx as usize) * 4;
                        if self.framebuffer[i] != 0xFF { return; }
                    }

                    let shade = self.shade((pal >> (id * 2)) & 0x03);
                    self.set_raw(tx as usize, current_ly as usize, shade);
                });
            });
    }

    #[inline(always)]
    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        self.set_raw(x, y, self.shade(color));
    }

    #[inline(always)]
    fn set_raw(&mut self, x: usize, y: usize, shade: u8) {
        let i = (y * 160 + x) * 4;
        self.framebuffer[i..i+3].fill(shade);
        self.framebuffer[i+3] = 0xFF;
    }

    fn shade(&self, color: u8) -> u8 {
        match color { 0 => 0xFF, 1 => 0xAA, 2 => 0x55, _ => 0x00 }
    }
}