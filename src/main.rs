mod cpu;
mod mmu;
mod registers;
mod ppu;

use cpu::CPU; 
use mmu::{MainBus, Bus};
use ppu::PPU;
use eframe::egui;

struct GBApp{
    cpu: CPU<MainBus>,
    ppu: PPU,
    last_ly: u8,
}

impl GBApp{
    fn new(rom_data: Vec<u8>) -> Self {
        let bus = MainBus::new(rom_data);
        let cpu = CPU::new(bus);
        let ppu = PPU::new();

        Self {
            cpu,
            ppu,
            last_ly: 255,
        }
    }
}

impl eframe::App for GBApp{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame){
        // run engine for 1 frame - 70224 cycles
        let mut frame_cycles = 0;
        let joyp = self.cpu.bus.read(0xFF00);
        let mut buttons = 0x0f; // 1 means not pressed

        ctx.input(|i|{
            // Bit 4 low = Directions
            if (joyp & 0x10) == 0 {
                if i.key_down(egui::Key::D)  {buttons &= !0x01;} // right
                if i.key_down(egui::Key::A)  {buttons &= !0x02;} // left
                if i.key_down(egui::Key::W)  {buttons &= !0x04;} // up
                if i.key_down(egui::Key::S)  {buttons &= !0x08;} // down
            }

            // Bit 5 low = Actions
            if (joyp & 0x20) == 0 {
                if i.key_down(egui::Key::Z) {buttons &= !0x01;} // A
                if i.key_down(egui::Key::X) {buttons &= !0x02;} // B
                if i.key_down(egui::Key::Space) {buttons &= !0x04;} // Select
                if i.key_down(egui::Key::Enter) {buttons &= !0x08;} // Start
            }
        });

        self.cpu.bus.write(0xFF00, (joyp & 0xF0) | buttons);


        while frame_cycles < 70224 {
            let d = if !self.cpu.halt {
                self.cpu.step()
            } else {
                self.cpu.tick();
                4
            };

            frame_cycles += d;
            let current_ly = self.cpu.bus.read(0xFF44);
            if current_ly < 144 && current_ly != self.last_ly {
                self.ppu.render_scanline(current_ly, &mut self.cpu.bus);
                self.last_ly = current_ly;
            }

    
            if current_ly == 153 { self.last_ly = 255; } // Reset at end of frame
        }
        // RENDER GUI
        egui::CentralPanel::default().show(ctx, |ui|{
            let pixels: Vec<u8> = self.ppu.frame.iter().flat_map(|&c| match c {
            0 => [224, 248, 208, 255], 
            1 => [136, 192, 112, 255],
            2 => [52, 104, 86, 255],  
            _ => [8, 24, 32, 255],
        }).collect();
        let screen_size = egui::vec2(160.0, 144.0) * 4.0;
        let texture = ctx.load_texture(
                "gb_screen", 
                egui::ColorImage::from_rgba_unmultiplied([160, 144], &pixels), 
                egui::TextureOptions::NEAREST
            );
        ui.image(egui::load::SizedTexture::new(texture.id(), screen_size));
        });
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {

    let rom_path = "pokemon_red.gb";
    let rom_data = std::fs::read(rom_path).expect("Failed to read ROM file");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "GB", 
        options,
        Box::new(|_| Ok(Box::new(GBApp::new(rom_data))))
    )
   
}

