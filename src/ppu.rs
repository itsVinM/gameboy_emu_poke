use crate::mmu::Mmu;

pub struct Ppu {
    pub framebuffer: [u8; 160 * 144 * 4], // RGBA
    pub dot: u32,
    pub ly:  u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            framebuffer: [0xFF; 160 * 144 * 4],
            dot: 0,
            ly:  0,
        }
    }

    pub fn tick(&mut self, cycles: u32, mmu: &mut Mmu) {
        // Update LY in IO so the CPU can read it
        mmu.io[0x44] = self.ly;

        let lcdc = mmu.io[0x40];
        if lcdc & 0x80 == 0 {
            // LCD off — reset
            self.ly  = 0;
            self.dot = 0;
            return;
        }

        self.dot += cycles;

        while self.dot >= 456 {
            self.dot -= 456;

            if self.ly < 144 {
                self.render_scanline(mmu);
            }

            self.ly += 1;

            if self.ly == 144 {
                mmu.io[0x0F] |= 0x01; // VBlank interrupt
            }
            if self.ly > 153 {
                self.ly = 0;
            }

            mmu.io[0x44] = self.ly;
        }
    }

    fn render_scanline(&mut self, mmu: &Mmu) {
        let lcdc = mmu.io[0x40];
        self.render_background(mmu, lcdc);
        self.render_window(mmu, lcdc);
        if lcdc & 0x02 != 0 {
            self.render_sprites(mmu);
        }
    }

    fn render_background(&mut self, mmu: &Mmu, lcdc: u8) {
        let scx = mmu.io[0x43];
        let scy = mmu.io[0x42];
        let bgp = mmu.io[0x47];

        for x in 0u8..160 {
            let px = x.wrapping_add(scx) as u16;
            let py = self.ly.wrapping_add(scy) as u16;

            let color = self.get_bg_pixel(mmu, lcdc, px, py, bgp, false);
            self.set_pixel(x as usize, self.ly as usize, color);
        }
    }

    fn render_window(&mut self, mmu: &Mmu, lcdc: u8) {
        if lcdc & 0x20 == 0 { return; } // window disabled

        let wy = mmu.io[0x4A];
        let wx = mmu.io[0x4B].wrapping_sub(7);
        let bgp = mmu.io[0x47];

        if self.ly < wy { return; }

        for x in 0u8..160 {
            if x < wx { continue; }
            let px = (x - wx) as u16;
            let py = (self.ly - wy) as u16;

            // Window always uses bit 6 of LCDC for tile map
            let map_base: u16 = if lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 };
            let color = self.get_bg_pixel(mmu, lcdc, px, py, bgp, true);
            let _ = map_base; // map_base used inside get_bg_pixel via lcdc flag
            self.set_pixel(x as usize, self.ly as usize, color);
        }
    }

    fn get_bg_pixel(&self, mmu: &Mmu, lcdc: u8, px: u16, py: u16, palette: u8, is_window: bool) -> u8 {
        let map_base: u16 = if is_window {
            if lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 }
        } else {
            if lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 }
        };

        let tile_idx = mmu.read(map_base + (py / 8) * 32 + (px / 8));

        let tile_addr: u16 = if lcdc & 0x10 != 0 {
            0x8000 + tile_idx as u16 * 16
        } else {
            (0x9000i32 + tile_idx as i8 as i32 * 16) as u16
        };

        let row_addr = tile_addr + (py % 8) * 2;
        let lo = mmu.read(row_addr);
        let hi = mmu.read(row_addr + 1);

        let bit = 7 - (px % 8) as u8;
        let color_id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
        (palette >> (color_id * 2)) & 0x03
    }

    fn render_sprites(&mut self, mmu: &Mmu) {
        let obp0 = mmu.io[0x48];
        let obp1 = mmu.io[0x49];

        // 40 sprites in OAM, 4 bytes each
        let mut count = 0;
        for i in 0..40 {
            let base = i * 4;
            let sprite_y = mmu.oam[base]     as i16 - 16;
            let sprite_x = mmu.oam[base + 1] as i16 - 8;
            let tile_idx = mmu.oam[base + 2];
            let attrs    = mmu.oam[base + 3];

            let ly = self.ly as i16;
            if ly < sprite_y || ly >= sprite_y + 8 { continue; }

            count += 1;
            if count > 10 { break; } // max 10 sprites per scanline

            let palette = if attrs & 0x10 != 0 { obp1 } else { obp0 };
            let x_flip  = attrs & 0x20 != 0;
            let y_flip  = attrs & 0x40 != 0;
            let priority = attrs & 0x80 != 0;

            let mut row = (ly - sprite_y) as u16;
            if y_flip { row = 7 - row; }

            let tile_addr = 0x8000u16 + tile_idx as u16 * 16 + row * 2;
            let lo = mmu.read(tile_addr);
            let hi = mmu.read(tile_addr + 1);

            for px in 0..8i16 {
                let screen_x = sprite_x + px;
                if screen_x < 0 || screen_x >= 160 { continue; }

                let bit = if x_flip { px } else { 7 - px } as u8;
                let color_id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                if color_id == 0 { continue; } // transparent

                // Priority: if bg pixel != 0 and sprite is behind, skip
                if priority {
                    let existing = self.framebuffer[(self.ly as usize * 160 + screen_x as usize) * 4];
                    if existing != 0xFF { continue; } // bg is not white, sprite behind
                }

                let shade = self.shade((palette >> (color_id * 2)) & 0x03);
                let i = (self.ly as usize * 160 + screen_x as usize) * 4;
                self.framebuffer[i]     = shade;
                self.framebuffer[i + 1] = shade;
                self.framebuffer[i + 2] = shade;
                self.framebuffer[i + 3] = 0xFF;
            }
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        let shade = self.shade(color);
        let i = (y * 160 + x) * 4;
        self.framebuffer[i]     = shade;
        self.framebuffer[i + 1] = shade;
        self.framebuffer[i + 2] = shade;
        self.framebuffer[i + 3] = 0xFF;
    }

    fn shade(&self, color: u8) -> u8 {
        match color {
            0 => 0xFF,
            1 => 0xAA,
            2 => 0x55,
            _ => 0x00,
        }
    }
}