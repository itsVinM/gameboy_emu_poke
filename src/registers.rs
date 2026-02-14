// src/resisters.rs
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Registers {
    pub a: u8, pub f:u8,
    pub b: u8, pub c:u8,
    pub d: u8, pub e:u8,
    pub h: u8, pub l:u8,
    pub sp: u16, pub pc: u16,
    pub ime: bool, pub halt: bool,
}

impl Registers {

    pub fn new() -> Self {
        Self {
            a: 0x01,
            f: 0xB0,        // Z, H, C flags set
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,    // Entry point of ROM
            ime: false,
            halt: false,
        }
    }
   
    // get 16 methods
    pub fn get_af(&self) -> u16 { u16::from_le_bytes([self.f, self.a]) }
    pub fn get_bc(&self) -> u16 { u16::from_le_bytes([self.c, self.b]) }
    pub fn get_de(&self) -> u16 { u16::from_le_bytes([self.e, self.d]) }
    pub fn get_hl(&self) -> u16 { u16::from_le_bytes([self.l, self.h]) }


    pub fn set_bc(&mut self, value: u16) { 
        let [c, b] = value.to_le_bytes(); 
        self.c = c; 
        self.b = b; 
    }
    pub fn set_de(&mut self, value: u16) { 
        let [e, d] = value.to_le_bytes(); 
        self.e = e; 
        self.d = d; 
    }
    pub fn set_hl(&mut self, value: u16) { 
        let [l, h] = value.to_le_bytes(); 
        self.l = l; 
        self.h = h; 
    }

    
    // FLAG GETTERS
    pub fn get_flag_z(&self) -> bool { (self.f & 0x80) != 0 }
    pub fn get_flag_n(&self) -> bool { (self.f & 0x40) != 0 }
    pub fn get_flag_h(&self) -> bool { (self.f & 0x20) != 0 }
    pub fn get_flag_c(&self) -> bool { (self.f & 0x10) != 0 }

    // FLAG SETTERS
    pub fn set_flag_z(&mut self, value: bool) { 
        self.f = if value { self.f | 0x80 } else { self.f & !0x80 }; 
    }
    pub fn set_flag_n(&mut self, value: bool) { 
        self.f = if value { self.f | 0x40 } else { self.f & !0x40 }; 
    }
    pub fn set_flag_h(&mut self, value: bool) { 
        self.f = if value { self.f | 0x20 } else { self.f & !0x20 }; 
    }
    pub fn set_flag_c(&mut self, value: bool) { 
        self.f = if value { self.f | 0x10 } else { self.f & !0x10 }; 
    }
    
    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.set_flag_z(z);
        self.set_flag_n(n);
        self.set_flag_h(h);
        self.set_flag_c(c);
    }
}