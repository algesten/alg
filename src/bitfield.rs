#[derive(Default)]
pub struct Bitfield(u32);
impl Bitfield {
    pub fn set(&mut self, bit: u8, on: bool) {
        if on {
            self.0 |= 1 << bit;
        } else {
            self.0 &= !(1 << bit);
        }
    }

    pub fn is(&self, bit: u8) -> bool {
        (self.0 & (1 << bit)) > 0
    }
}

impl core::ops::Deref for Bitfield {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
