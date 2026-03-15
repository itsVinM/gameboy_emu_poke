use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, ImageData};

pub mod cpu;
pub mod mmu;
pub mod ppu;
pub mod registers;

use cpu::Cpu;
use mmu::Mmu;
use ppu::Ppu;

// Constant for Game Boy frame timing
pub const MAX_FRAME_CYCLES: u32 = 70224;

#[wasm_bindgen]
pub struct EmulatorState {
    cpu: Cpu,
    ppu: Ppu,
    mmu: Mmu,
    div_acc: u32,
    timer_acc: u32,
}

#[wasm_bindgen]
impl EmulatorState {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: Vec<u8>) -> Self {
        // Redirect Rust panics to the browser console for easier debugging
        console_error_panic_hook::set_once();
        
        Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            mmu: Mmu::new(rom, vec![0u8; 0x8000]),
            div_acc: 0,
            timer_acc: 0,
        }
    }

    /// Executes one full frame of Game Boy logic (~16.7ms)
    pub fn tick_frame(&mut self) {
        let mut frame_cycles = 0;

        while frame_cycles < MAX_FRAME_CYCLES {
            let s = self.cpu.step(&mut self.mmu);
            self.ppu.tick(s, &mut self.mmu);

            // --- DIVIDER (DIV) Logic ---
            self.div_acc += s;
            if self.div_acc >= 256 {
                self.div_acc -= 256;
                // DIV is at 0xFF04. Wrapping_add simulates hardware register behavior.
                self.mmu.io[0x04] = self.mmu.io[0x04].wrapping_add(1);
            }

            // --- TIMER Logic (Dialogue/Delay Driver) ---
            let tac = self.mmu.read(0xFF07);
            if tac & 0x04 != 0 { // Timer is enabled
                self.timer_acc += s;
                let threshold = match tac & 0x03 {
                    0x00 => 1024, // 4096 Hz
                    0x01 => 16,   // 262144 Hz
                    0x02 => 64,   // 65536 Hz
                    0x03 => 256,  // 16384 Hz
                    _ => 1024,
                };

                while self.timer_acc >= threshold {
                    self.timer_acc -= threshold;
                    let tima = self.mmu.read(0xFF05);
                    if tima == 0xFF {
                        // Overflow: Reload from TMA (0xFF06) and trigger IRQ (Bit 2)
                        self.mmu.write(0xFF05, self.mmu.read(0xFF06));
                        let if_val = self.mmu.read(0xFF0F);
                        self.mmu.write(0xFF0F, if_val | 0x04);
                    } else {
                        self.mmu.write(0xFF05, tima + 1);
                    }
                }
            }
            frame_cycles += s;
        }
    }

    /// Returns a pointer to the PPU framebuffer for zero-copy drawing in JS
    pub fn framebuffer_ptr(&self) -> *const u8 {
        self.ppu.framebuffer.as_ptr()
    }

    #[wasm_bindgen]
    pub fn save_wasm(&self) -> Vec<u8> {
        self.mmu.get_save_data()
    }

    #[wasm_bindgen]
    pub fn load_save_wasm(&mut self, data: Vec<u8>) {
        self.mmu.load_save_data(data);
    }

    /// Updates Joypad state from JavaScript key events
    /// dpad_mask and button_mask should be passed as bitflags (Active Low)
    pub fn update_joypad(&mut self, d_pad: u8, buttons: u8) {
        // Calculate transition for Joypad Interrupt
        let select = self.mmu.io[0x00] & 0x30;
        let mut current_joyp = 0x0F;
        
        if (select & 0x10) == 0 { current_joyp &= d_pad; }
        if (select & 0x20) == 0 { current_joyp &= buttons; }

        if (self.mmu.prev_joyp & !current_joyp) & 0x0F != 0 {
            let if_val = self.mmu.read(0xFF0F);
            self.mmu.write(0xFF0F, if_val | 0x10);
        }

        self.mmu.dpad = d_pad;
        self.mmu.buttons = buttons;
        self.mmu.prev_joyp = current_joyp;
    }
}