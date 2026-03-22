#[cfg(not(target_arch = "wasm32"))]
use minifb::{Key, KeyRepeat, Window, WindowOptions};
#[cfg(not(target_arch = "wasm32"))]
use pokegameboy::{cpu::Cpu, mmu::Mmu, ppu::Ppu, MAX_FRAME_CYCLES};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let rom = std::fs::read("rom.gb").expect("rom.gb missing");
    let mut mmu = Mmu::new(rom, vec![0u8; 0x8000]);
    
    // --- LOAD SAVE DATA ---
    if let Ok(save_data) = std::fs::read("rom.sav") {
        mmu.load_save_data(save_data);
        println!("Principal: Existing save state loaded from rom.sav");
    }

    let (mut cpu, mut ppu) = (Cpu::new(), Ppu::new());

    let (w, h, sc) = (160, 144, 4);
    let mut window = Window::new("PokéGB Principal Build", w * sc, h * sc, WindowOptions::default()).unwrap();
    window.limit_update_rate(Some(std::time::Duration::from_micros(8333))); 

    let mut fb = vec![0u32; (w * sc) * (h * sc)];
    let mut paused = false;
    
    let mut div_acc: u32 = 0;
    let mut timer_acc: u32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Space, KeyRepeat::No) { paused = !paused; }

        // ---  MANUAL SAVE TRIGGER ---
        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
            let data = mmu.get_save_data();
            std::fs::write("rom.sav", data).expect("Failed to write save file");
            println!("Principal: Manual save successful (rom.sav)");
        }

        if !paused {
            update_joypad(&window, &mut mmu);
            
            let mut frame_cycles = 0;
            while frame_cycles < MAX_FRAME_CYCLES {
                let s = cpu.step(&mut mmu);
                ppu.tick(s, &mut mmu);
                
                div_acc += s;
                if div_acc >= 256 {
                    div_acc -= 256;
                    mmu.io[0x04] = mmu.io[0x04].wrapping_add(1);
                }

                let tac = mmu.read(0xFF07);
                if tac & 0x04 != 0 {
                    timer_acc += s;
                    let threshold = match tac & 0x03 {
                        0x00 => 1024, 0x01 => 16, 0x02 => 64, 0x03 => 256,
                        _ => 1024,
                    };

                    while timer_acc >= threshold {
                        timer_acc -= threshold;
                        let tima = mmu.read(0xFF05);
                        if tima == 0xFF {
                            mmu.write(0xFF05, mmu.read(0xFF06)); 
                            let if_val = mmu.read(0xFF0F);
                            mmu.write(0xFF0F, if_val | 0x04); 
                        } else {
                            mmu.write(0xFF05, tima + 1);
                        }
                    }
                }
                frame_cycles += s;
            }
        }

        window.set_title(&format!(
            "PokéGB | PC:{:04X} | LY:{:02X} | IF:{:02X} | IE:{:02X}",
            cpu.regs.pc, mmu.read(0xFF44), mmu.read(0xFF0F), mmu.read(0xFFFF)
        ));

        render_frame(&mut fb, &ppu, w, h, sc);
        window.update_with_buffer(&fb, w * sc, h * sc).unwrap();
    }

    // --- 🏛️ AUTO-SAVE ON EXIT ---
    let data = mmu.get_save_data();
    let _ = std::fs::write("rom.sav", data);
    println!("Principal: Shutdown successful. Auto-save completed.");
}

#[cfg(not(target_arch = "wasm32"))]
fn render_frame(fb: &mut [u32], ppu: &Ppu, w: usize, h: usize, sc: usize) {
    for y in 0..h {
        let src_row = y * 640;
        for x in 0..w {
            let i = src_row + (x * 4);
            let color = ((ppu.framebuffer[i] as u32) << 16) | 
                        ((ppu.framebuffer[i+1] as u32) << 8) | 
                         (ppu.framebuffer[i+2] as u32);
            
            for dy in 0..sc {
                let start = ((y * sc + dy) * (w * sc)) + (x * sc);
                fb[start..start + sc].fill(color);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn update_joypad(w: &Window, m: &mut Mmu) {
    let mut d = 0x0F; // D-pad: Down, Up, Left, Right
    let mut b = 0x0F; // Buttons: Start, Select, B, A

    // D-Pad mapping
    if w.is_key_down(Key::Down)  { d &= !0x08; }
    if w.is_key_down(Key::Up)    { d &= !0x04; }
    if w.is_key_down(Key::Left)  { d &= !0x02; }
    if w.is_key_down(Key::Right) { d &= !0x01; }
    
    // Face button mappign
    if w.is_key_down(Key::Enter) { b &= !0x08; } // Start 
    if w.is_key_down(Key::S)     { b &= !0x04; } // Select
    if w.is_key_down(Key::B)     { b &= !0x02; } // Map B
    if w.is_key_down(Key::A)     { b &= !0x01; } // Map A

    let select = m.io[0x00] & 0x30;
    let mut current_joyp = 0x0F;
    if (select & 0x10) == 0 { current_joyp &= d; }
    if (select & 0x20) == 0 { current_joyp &= b; }

    if (m.prev_joyp & !current_joyp) & 0x0F != 0 {
        let if_val = m.read(0xFF0F);
        m.write(0xFF0F, if_val | 0x10);
    }

    m.dpad = d;
    m.buttons = b;
    m.prev_joyp = current_joyp;
}