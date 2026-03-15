use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, ImageData};

pub mod cpu;
pub mod mmu;
pub mod ppu;
pub mod registers;

use cpu::Cpu;
use mmu::Mmu;
use ppu::Ppu;

pub const MAX_FRAME_CYCLES: u32 = 70224;

#[wasm_bindgen]
pub struct WebEmulator {
    cpu: Cpu,
    mmu: Mmu,
    ppu: Ppu,
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: Vec<u8>) -> Self {
        let extram = vec![0u8; 0x8000]; // Save logic handled via LocalStorage in JS
        Self {
            cpu: Cpu::new(),
            mmu: Mmu::new(rom, extram),
            ppu: Ppu::new(),
        }
    }

    pub fn tick_frame(&mut self) {
        let mut frame_cycles = 0u32;
        // 70224 cycles is exactly one GameBoy frame
        while frame_cycles < MAX_FRAME_CYCLES {
            let cycles = self.cpu.step(&mut self.mmu);
            self.ppu.tick(cycles, &mut self.mmu);
            frame_cycles += cycles;
        }
    }

    pub fn get_framebuffer(&self) -> *const u8 {
        self.ppu.framebuffer.as_ptr()
    }

    pub fn update_joypad(&mut self, key_code: u8, pressed: bool) {
       
    }
}

