// src/resisters.rs

#[allow(dead_code)]
pub struct Registers {
    pub a: u8, pub f:u8,
    pub b: u8, pub c:u8,
    pub d: u8, pub e:u8,
    pub h: u8, pub l:u8,
    pub sp: u16, pub pc: u16,
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
        }
    }
    // making the 16-bit view
    pub fn get_af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f])
    }

    pub fn get_bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    pub fn get_de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    pub fn get_hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_af(&mut self, value: u16){
        let bytes = value.to_be_bytes(); 
        self.a = bytes[0];
        self.f = bytes[1] & 0xF0;
    } 

    pub fn set_bc(&mut self, value: u16){
        let bytes = value.to_be_bytes(); 
        self.b = bytes[0];
        self.c = bytes[1];
    } 

    pub fn set_de(&mut self, value: u16){
        let bytes = value.to_be_bytes(); 
        self.d = bytes[0];
        self.e = bytes[1];
    } 

    pub fn set_hl(&mut self, value: u16){
        let bytes = value.to_be_bytes(); 
        self.h = bytes[0];
        self.l = bytes[1];
    } 


    // FLAG BIT POSITION
    // Z: 7, N: 6 , H: 5, C: 4
    pub fn get_flag_z(&self) -> bool { (self.f & 0x80) != 0}
    pub fn get_flag_n(&self) -> bool { (self.f & 0x40) != 0}
    pub fn get_flag_h(&self) -> bool { (self.f & 0x20) != 0}
    pub fn get_flag_c(&self) -> bool { (self.f & 0x10) != 0}

    pub fn set_flag_z(&mut self, value: bool){
        if value { self.f |= 0x80} else { self.f &= !0x80}
    }
    pub fn set_flag_n(&mut self, value: bool){
        if value { self.f |= 0x40} else { self.f &= !0x40}
    }
    pub fn set_flag_h(&mut self, value: bool){
        if value { self.f |= 0x20} else { self.f &= !0x20}
    }
    pub fn set_flag_c(&mut self, value: bool){
        if value { self.f |= 0x10} else { self.f &= !0x10}
    }
}