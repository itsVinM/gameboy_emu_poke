use crate::registers::Registers;
use crate::mmu::Bus;

pub struct CPU<B: Bus>{
    pub reg: Registers,
    pub bus: B,
    pub cycles: u32,
    pub halt: bool,
    pub ime: bool, 
    pub ppu_dots: u16,
    pub ppu_ly: u8,
}

impl<B: Bus> CPU<B>{
    pub fn new(bus: B)-> Self{
        Self{
            reg: Registers::new(),
            bus,
            cycles: 0,
            halt: false,
            ime: false,
            ppu_dots: 0,
            ppu_ly: 0,
        }
    }

    pub fn read8(&mut self, addr: u16) -> u8 {
        self.tick(); 
        self.bus.read(addr)
    }

    pub fn write8(&mut self, addr: u16, val: u8) {
        self.tick(); // take 4 cycles every read
        self.bus.write(addr, val);
    
    }

    pub fn tick(&mut self){
        self.cycles+=4;   
        self.ppu_dots += 4;
       
        if self.ppu_dots >= 456 {
            self.ppu_dots -= 456;
            
            // Use wrapping_add to handle the 154 reset logic
            self.ppu_ly = (self.ppu_ly + 1) % 154;

            self.bus.write(0xFF44, self.ppu_ly);

            if self.ppu_ly == 144 {
                // Trigger V-Blank interrupt
                let current_if = self.bus.read(0xFF0F);
                self.bus.write(0xFF0F, current_if | 0x01);
            }
    }
    
    }

    // read bytes at PC and increments it
    pub fn fetch(&mut self) -> u8{
        let opcode = self.bus.read(self.reg.pc);
        self.reg.pc = self.reg.pc.wrapping_add(1);
        self.tick(); //fetching byte takes time
        opcode
    }

    // helper fetch to read 16-bit values (Little Endian)
    pub fn fetch_u16(&mut self) -> u16{
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        (high << 8)|low
    }

    pub fn get_reg8(&mut self, index: u8) -> u8 {
        match index {
            0 => self.reg.b,
            1 => self.reg.c,
            2 => self.reg.d,
            3 => self.reg.e,
            4 => self.reg.h,
            5 => self.reg.l,
            6 => self.read8(self.reg.get_hl()), // This handles the [HL] columns!
            7 => self.reg.a,
            _ => unreachable!(),
        }
    }

    pub fn set_reg8(&mut self, index: u8, val: u8) {
        match index {
            0 => self.reg.b = val,
            1 => self.reg.c = val,
            2 => self.reg.d = val,
            3 => self.reg.e = val,
            4 => self.reg.h = val,
            5 => self.reg.l = val,
            6 => self.write8(self.reg.get_hl(), val), // Write to memory at [HL]
            7 => self.reg.a = val,
            _ => unreachable!(),
        }
    }

    pub fn push_u16(&mut self, val: u16){
        let high = (val >> 8) as u8;
        let low = (val & 0xFF) as u8;
        self.reg.sp = self.reg.sp.wrapping_sub(1);
        self.write8(self.reg.sp, high);
        self.reg.sp = self.reg.sp.wrapping_sub(1);
        self.write8(self.reg.sp, low);
    }

    pub fn pop_u16(&mut self) -> u16{
        let low = self.read8(self.reg.sp);
        self.reg.sp = self.reg.sp.wrapping_add(1);
        let high = self.read8(self.reg.sp);
        self.reg.sp = self.reg.sp.wrapping_add(1);
        // combine as Little Endian
        ((high as u16) << 8) | (low as u16)
    }


    fn get_r16(&self, idx: u8) -> u16 {
        match idx & 3 {
            0 => self.reg.get_bc(),
            1 => self.reg.get_de(),
            2 => self.reg.get_hl(),
            3 => self.reg.sp,
            _ => unreachable!(),
        }
    }

    fn set_r16(&mut self, idx: u8, val: u16) {
        match idx & 3 {
            0 => self.reg.set_bc(val),
            1 => self.reg.set_de(val),
            2 => self.reg.set_hl(val),
            3 => self.reg.sp = val,
            _ => unreachable!(),
        }
    }

        // Helper for PUSH/POP (where index 3 is AF)
    fn get_r16_stack(&self, idx: u8) -> u16 {
            if idx & 3 == 3 { self.reg.get_af() } else { self.get_r16(idx) }
        }

    fn set_r16_stack(&mut self, idx: u8, val: u16) {
            if idx & 3 == 3 { self.reg.set_af(val) } else { self.set_r16(idx, val) }
    }

    fn handle_interrupts(&mut self) {
        let ie = self.bus.read(0xFFFF);
        let if_flag = self.bus.read(0xFF0F);
        let pending = ie & if_flag;

        // We only service if IME is true AND there is a pending request
        if self.ime && pending != 0 {
            self.halt = false; 
            self.ime = false; // Disable IME while servicing

            if (pending & 0x01) != 0 { // V-Blank priority
                self.service_interrupt(0, 0x0040);
            }
            // Add other interrupts (Stat, Timer, etc) here later
        }
    }

    fn service_interrupt(&mut self, bit: u8, vector: u16) {
        // Clear the request bit in IF
        let if_val = self.bus.read(0xFF0F);
        self.bus.write(0xFF0F, if_val & !(1 << bit));

        // Save current PC and jump to the game's interrupt handler
        let pc = self.reg.pc;
        self.push_u16(pc);
        self.reg.pc = vector;
        
        // Servicing an interrupt takes roughly 20 cycles total
        for _ in 0..5 { self.tick(); } 
    }

    pub fn execute_alu(&mut self, op: u8, val: u8) {
        let a = self.reg.a;
        match op {
            0 => { // ADD
                let (res, c) = a.overflowing_add(val);
                self.set_f(res, 0, (a & 0xF) + (val & 0xF) > 0xF, c);
                self.reg.a = res;
            }
            // ADC (1)
            1 => { 
                let c_in = if self.reg.get_flag_c() { 1 } else { 0 };
                let res = a.wrapping_add(val).wrapping_add(c_in);
                // Flags based on original 'a', 'val', and 'c_in'
                let h = (a & 0xF) + (val & 0xF) + c_in > 0xF;
                let c_out = (a as u16 + val as u16 + c_in as u16) > 0xFF;
                self.reg.a = res;
                self.set_f(res, 0, h, c_out);
            }
            2 | 7 => { // SUB or CP
                let (res, c) = a.overflowing_sub(val);
                self.set_f(res, 1, (a & 0xF) < (val & 0xF), c);
                if op == 2 { self.reg.a = res; }
            }
            3 => { // SBC
                let c_in = if self.reg.get_flag_c() { 1 } else { 0 };
                let res = a.wrapping_sub(val).wrapping_sub(c_in);
                // Flags based on original 'a', 'val', and 'c_in'
                let h = (a as i16 & 0xF) < (val as i16 & 0xF) + c_in as i16;
                let c_out = (a as u16) < (val as u16 + c_in as u16);
                self.reg.a = res;
                self.set_f(res, 1, h, c_out);
            }
            4 => { self.reg.a &= val; self.set_f(self.reg.a, 0, 1, 0); } // AND
            5 => { self.reg.a ^= val; self.set_f(self.reg.a, 0, 0, 0); } // XOR
            6 => { self.reg.a |= val; self.set_f(self.reg.a, 0, 0, 0); } // OR
            _ => unreachable!(),
        }
    }

    // Compact flag setter: Z N H C
    fn set_f(&mut self, res: u8, n: u8, h: impl Into<u8>, c: impl Into<u8>) {
        self.reg.set_flag_z(res == 0);
        self.reg.set_flag_n(n != 0);
        self.reg.set_flag_h(h.into() != 0);
        self.reg.set_flag_c(c.into() != 0);
    }

    pub fn step(&mut self) -> u32 {
        
        self.handle_interrupts();
        
        if self.halt {
            self.tick(); // Still consume cycles while halted
            return self.cycles - self.cycles; // Simplified for cycle counting
        }

        let start_cycles = self.cycles;
        let opcode = self.fetch();
        
        if self.cycles % 5000 == 0 {
            println!("STUCK AT PC: {:#06X} | Opcode: {:#04X} | LY: {}", 
                self.reg.pc, 
                self.bus.read(self.reg.pc),
                self.bus.read(0xFF44)
            );
        }
        
        match opcode {
            0x00 => {} // NOP
            0x10 => { self.fetch(); } // STOP
            0x76 => self.halt = true, // HALT

            // --- 8-bit Loads (LD r, r) ---
            0x40..=0x7F => {
                let v = self.get_reg8(opcode & 7); 
                self.set_reg8((opcode >> 3) & 7, v); 
            }

            // --- 8-bit Immediate Loads (LD r, n8) ---
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E | 0x36 => {
                let val = self.fetch();
                let dest = (opcode >> 3) & 7;
                if dest == 6 { self.write8(self.reg.get_hl(), val); } 
                else { self.set_reg8(dest, val); }
            }

            // --- 16-bit Immediate Loads ---
            0x01 | 0x11 | 0x21 | 0x31 => { 
                let v = self.fetch_u16(); 
                self.set_r16(opcode >> 4, v); 
            }

            // --- 8-bit ALU (Arithmetic/Logic) ---
            0x80..=0xBF => {
                let val = self.get_reg8(opcode & 7);
                self.execute_alu((opcode >> 3) & 7, val);
            }
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let v = self.fetch(); 
                self.execute_alu((opcode >> 3) & 7, v);
            }

            // --- INC/DEC r8 ---
            0x04 | 0x05 | 0x0C | 0x0D | 0x14 | 0x15 | 0x1C | 0x1D |
            0x24 | 0x25 | 0x2C | 0x2D | 0x34 | 0x35 | 0x3C | 0x3D => {
                let r = (opcode >> 3) & 7;
                let v = self.get_reg8(r);
                let is_dec = (opcode & 1) != 0;
                let res = if is_dec { v.wrapping_sub(1) } else { v.wrapping_add(1) };
                self.set_reg8(r, res);
                self.reg.set_flag_z(res == 0);
                self.reg.set_flag_n(is_dec);
                self.reg.set_flag_h(if is_dec { (v & 0xF) == 0 } else { (v & 0xF) == 0xF });
            }

            // --- 16-bit Arithmetic (INC/DEC/ADD) ---
            0x03 | 0x13 | 0x23 | 0x33 | 0x0B | 0x1B | 0x2B | 0x3B => {
                self.tick();
                let is_dec = (opcode & 8) != 0;
                let r_idx = (opcode >> 4) & 3;
                let v = self.get_r16(r_idx);
                self.set_r16(r_idx, if is_dec { v.wrapping_sub(1) } else { v.wrapping_add(1) });
            }
            0x09 | 0x19 | 0x29 | 0x39 => { // ADD HL, r16
                self.tick();
                let hl = self.reg.get_hl();
                let val = self.get_r16(opcode >> 4);
                let res = hl.wrapping_add(val);
                self.reg.set_hl(res);
                self.reg.set_flag_n(false);
                self.reg.set_flag_h((hl & 0xFFF) + (val & 0xFFF) > 0xFFF);
                self.reg.set_flag_c((hl as u32 + val as u32) > 0xFFFF);
            }

            // --- Jumps, Calls, & Returns ---
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 => { // JR
                let e = self.fetch() as i8;
                if opcode == 0x18 || self.check_cond(opcode) { self.reg.pc = self.reg.pc.wrapping_add_signed(e as i16); self.tick(); }
            }
            0xC3 | 0xC2 | 0xCA | 0xD2 | 0xDA => { // JP
                let nn = self.fetch_u16();
                if opcode == 0xC3 || self.check_cond(opcode) { self.reg.pc = nn; self.tick(); }
            }
            0xCD | 0xC4 | 0xCC | 0xD4 | 0xDC => { // CALL
                let nn = self.fetch_u16();
                if opcode == 0xCD || self.check_cond(opcode) { self.push_u16(self.reg.pc); self.reg.pc = nn; self.tick(); }
            }
            0xC9 | 0xC0 | 0xC8 | 0xD0 | 0xD8 => { // RET
                if opcode != 0xC9 { self.tick(); }
                if opcode == 0xC9 || self.check_cond(opcode) { self.reg.pc = self.pop_u16(); self.tick(); }
            }
            0xD9 => { // RETI
                let old_pc = self.reg.pc;
                self.reg.pc = self.pop_u16(); 
                self.ime = true; 
                self.tick(); 
                println!(">>> RETI: Returning from {:#06X} to {:#06X}. IME is now true. <<<", old_pc, self.reg.pc);
            
            } 
            0xE9 => self.reg.pc = self.reg.get_hl(), // JP HL
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => { // RST
                self.push_u16(self.reg.pc);
                self.reg.pc = (opcode & 0x38) as u16;
                self.tick();
            }

            // --- Stack Operations ---
            0xC5 | 0xD5 | 0xE5 | 0xF5 => { self.tick(); self.push_u16(self.get_r16_stack(opcode >> 4 & 3)); }
            0xC1 | 0xD1 | 0xE1 | 0xF1 => { let v = self.pop_u16(); self.set_r16_stack(opcode >> 4 & 3, v); }

            // --- Memory I/O (Crucial for graphics/input) ---
            0xE0 | 0xF0 => {
                let a = 0xFF00 | self.fetch() as u16;
                if opcode == 0xE0 { self.write8(a, self.reg.a); } else { self.reg.a = self.read8(a); }
            }
            0xE2 | 0xF2 => {
                let a = 0xFF00 | self.reg.c as u16;
                if opcode == 0xE2 { self.write8(a, self.reg.a); } else { self.reg.a = self.read8(a); }
            }
            0xEA | 0xFA => {
                let a = self.fetch_u16();
                if opcode == 0xEA { self.write8(a, self.reg.a); } else { self.reg.a = self.read8(a); }
            }
            0x22 | 0x32 | 0x2A | 0x3A => {
                let hl = self.reg.get_hl();
                if (opcode & 0x08) != 0 { // 0x2A and 0x3A are loads into A
                    self.reg.a = self.read8(hl); 
                } else { 
                    self.write8(hl, self.reg.a); 
                }
                // Increment for 0x22/0x2A, Decrement for 0x32/0x3A
                let next_hl = if (opcode & 0x10) == 0 { hl.wrapping_add(1) } else { hl.wrapping_sub(1) };
                self.reg.set_hl(next_hl);
            }
            0x02 | 0x12 => self.write8(self.get_r16(opcode >> 4), self.reg.a),
            0x0A | 0x1A => self.reg.a = self.read8(self.get_r16(opcode >> 4)),

            // --- 0xCB Prefix (Bit-shifts & BIT checks) ---
            0xCB => {
                let cb_opcode = self.fetch();
                let r = cb_opcode & 7;
                let bit = (cb_opcode >> 3) & 7;
                let v = self.get_reg8(r);
                match cb_opcode >> 6 {
                    0 => { 
                    
                        let mut val = v;
                        let old_c = self.reg.get_flag_c();
                        
                        match bit {
                            0 => { // RLC (Rotate Left)
                                let c = val >> 7;
                                val = (val << 1) | c;
                                self.reg.set_flag_c(c == 1);
                            }
                            1 => { // RRC (Rotate Right)
                                let c = val & 1;
                                val = (val >> 1) | (c << 7);
                                self.reg.set_flag_c(c == 1);
                            }
                            2 => { // RL (Rotate Left through Carry)
                                let c = if old_c { 1 } else { 0 };
                                let next_c = val >> 7;
                                val = (val << 1) | c;
                                self.reg.set_flag_c(next_c == 1);
                            }
                            3 => { // RR (Rotate Right through Carry)
                                let c = if old_c { 1 } else { 0 };
                                let next_c = val & 1;
                                val = (val >> 1) | (c << 7);
                                self.reg.set_flag_c(next_c == 1);
                            }
                            4 => { // SLA (Shift Left Arithmetic)
                                self.reg.set_flag_c((val >> 7) == 1);
                                val <<= 1;
                            }
                            5 => { // SRA (Shift Right Arithmetic - Keep MSB)
                                self.reg.set_flag_c((val & 1) == 1);
                                val = (val as i8 >> 1) as u8;
                            }
                            6 => { // SWAP (Swap nibbles)
                                val = (val << 4) | (val >> 4);
                                self.reg.set_flag_c(false);
                            }
                            7 => { // SRL (Shift Right Logical)
                                self.reg.set_flag_c((val & 1) == 1);
                                val >>= 1;
                            }
                            _ => unreachable!(),
                        }

                        self.set_reg8(r, val);
                        self.reg.set_flag_z(val == 0);
                        self.reg.set_flag_n(false);
                        self.reg.set_flag_h(false);
                    }
                    
                    1 => { self.reg.set_flag_z((v & (1 << bit)) == 0); self.reg.set_flag_n(false); self.reg.set_flag_h(true); }
                    2 => self.set_reg8(r, v & !(1 << bit)),
                    3 => self.set_reg8(r, v | (1 << bit)),
                    _ => unreachable!()
                }
            }

            // --- Specialized (DAA is vital for Pokemon scores/stats) ---
            0x27 => { 
                let mut a = self.reg.a as u16;
                if !self.reg.get_flag_n() {
                    if self.reg.get_flag_h() || (a & 0x0F) > 0x09 { a += 0x06; }
                    if self.reg.get_flag_c() || a > 0x9F { a += 0x60; self.reg.set_flag_c(true); }
                } else {
                    if self.reg.get_flag_h() { a = a.wrapping_sub(0x06); }
                    if self.reg.get_flag_c() { a = a.wrapping_sub(0x60); }
                }
                self.reg.a = a as u8;
                self.reg.set_flag_z(self.reg.a == 0); self.reg.set_flag_h(false);
            }
            0xF3 => self.ime = false,
            0xFB => self.ime = true,
            0x2F => { self.reg.a = !self.reg.a; self.reg.set_flag_n(true); self.reg.set_flag_h(true); }
            0x37 => { self.reg.set_flag_n(false); self.reg.set_flag_h(false); self.reg.set_flag_c(true); }
            0x3F => { self.reg.set_flag_n(false); self.reg.set_flag_h(false); self.reg.set_flag_c(!self.reg.get_flag_c()); }
            // LD (nn), SP
            0x08 => {
                let addr = self.fetch_u16();
                // Write the low byte first, then the high byte (Little Endian)
                self.write8(addr, (self.reg.sp & 0xFF) as u8);
                self.write8(addr.wrapping_add(1), (self.reg.sp >> 8) as u8);
            }
            _ => panic!("Opcode {:#04X} required for Pokemon Red/Blue not yet optimized!", opcode),
        }
        
        
        self.cycles - start_cycles
    }
    fn check_cond(&self, opcode: u8) -> bool {
        match (opcode >> 3) & 3 {
            0 => !self.reg.get_flag_z(), // NZ
            1 => self.reg.get_flag_z(),  // Z
            2 => !self.reg.get_flag_c(), // NC
            3 => self.reg.get_flag_c(),  // C
            _ => false
        }
    }       
}
