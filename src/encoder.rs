//! A twiddly knob encoder reader.
//!
//! This works using a "quadrature".
//!
//! ```ignore
//!    +----+    +----+    
//!    |    |    |    |       A
//!  --+    +----+    +----
//!       +----+    +----+    
//!       |    |    |    |    B
//!   ----+    +----+    +--
//!        ^  ^ ^  ^
//!        1  2 3  4
//! ```
//!
//! The states are `AB`:
//!
//! 1. is `11`
//! 2. is `01`
//! 3. is `00`
//! 4. is `10`
//!
//! The only valid transitions clock-wise are:
//!
//! * `11` -> `01`
//! * `01` -> `00`
//! * `00` -> `10`
//! * `10` -> `11`
//!
//! And the reverse counter clock wise.
//!
//! * `01` -> `11`
//! * `00` -> `01`
//! * `10` -> `00`
//! * `11` -> `10`
//!
//! We can make pairs to create a lookup table of valid pairs `1101`, `0100`, and -1 or 1 to denote direction.

const TABLE: [i8; 16] = [
    0,  // 0000
    -1, // 0001
    1,  // 0010
    0,  // 0011
    1,  // 0100
    0,  // 0101
    0,  // 0110
    -1, // 0111
    -1, // 1000
    0,  // 1001
    0,  // 1010
    1,  // 1011
    0,  // 1100
    1,  // 1101
    -1, // 1110
    0,  // 1111
];

use core::ops::BitAnd;

use crate::clock::Time;
use crate::input::DeltaInput;

/// Abstraction of something providing two pins as source for a "qaudrature", which
/// is the fancy word for how encoders encode.
pub trait QuadratureSource {
    fn pin_a(&self) -> bool;
    fn pin_b(&self) -> bool;
}

/// An encoder works over some QuadratureSource to provide a DeltaInput CW and CCW movement.
pub struct Encoder<T> {
    quad: T,
    prev_next: u8,
    state: u8,
    last_pos: isize,
}

impl<T> Encoder<T> {
    /// Create an encoder over some quadrature source.
    pub fn new(quad: T) -> Self {
        Encoder {
            quad,
            prev_next: 0,
            state: 0,
            last_pos: 4,
        }
    }
}

impl<T, const CLK: u32> DeltaInput<CLK> for Encoder<T>
where
    T: QuadratureSource,
{
    fn tick(&mut self, _now: Time<CLK>) -> i8 {
        let mut cur = 0;

        if self.quad.pin_a() {
            cur |= 0b10;
        }

        if self.quad.pin_b() {
            cur |= 0b01;
        }

        if (self.prev_next & 0b11) == cur {
            // no change
            return 0;
        }

        // Rotate up last read 2 bits and discard the rest.
        self.prev_next = (self.prev_next << 2) & 0b1100;

        // insert new reading
        self.prev_next |= cur;

        let direction = TABLE[self.prev_next as usize];

        if direction != 0 {
            // Move current state up to make state for new, and put in the new.
            self.state = (self.state << 4) | self.prev_next;

            // The possible state to observe when going CCW
            const STATE_CCW: &[u8] = &[0b10000001, 0b00010111, 0b01111110, 0b11101000];

            // The possible state to observe when going CW
            const STATE_CW: &[u8] = &[0b01000010, 0b00101011, 0b10111101, 0b11010100];

            let pos = {
                if let Some(pos) = STATE_CW.iter().position(|s| s == &self.state) {
                    Some(pos as isize + 1)
                } else if let Some(pos) = STATE_CCW.iter().position(|s| s == &self.state) {
                    Some(-1 * pos as isize - 1)
                } else {
                    None
                }
            };

            if let Some(pos) = pos {
                // If the polarity changes or we jump to a lower position in the
                // table, we are on a new turn.
                let is_new_turn = pos.abs() == 1 && self.last_pos.abs() != 1;

                self.last_pos = pos;

                if is_new_turn {
                    return pos.signum() as i8;
                }
            }
        }

        0
    }
}

/// A quadrature source reading from a shared number (such as u16) and
/// providing the two pins with bitmasks against it.
///
/// This requires W to be zero by default.
///
/// The pointer to a word given must be possible to dereference forever.
#[derive(Clone)]
pub struct BitmaskQuadratureSource<W> {
    word: *const W,
    zero: W,
    mask_a: W,
    mask_b: W,
}

impl<W> BitmaskQuadratureSource<W>
where
    W: BitAnd<Output = W> + Default + Copy + PartialOrd,
{
    pub fn new(word: *const W, mask_a: W, mask_b: W) -> Self {
        BitmaskQuadratureSource {
            word,
            zero: W::default(),
            mask_a,
            mask_b,
        }
    }
}

impl<W> QuadratureSource for BitmaskQuadratureSource<W>
where
    W: BitAnd<Output = W> + Default + Copy + PartialOrd,
{
    fn pin_a(&self) -> bool {
        (unsafe { *self.word }).bitand(self.mask_a) > self.zero
    }

    fn pin_b(&self) -> bool {
        (unsafe { *self.word }).bitand(self.mask_b) > self.zero
    }
}

/// Accelerator of encoders.
pub struct EncoderAccelerator<E, const CLK: u32> {
    /// Encoder to read impulses from.a
    encoder: E,
    /// This keeps the previous reading.,
    prev: Reading<CLK>,
    /// Current speed in millionths per milliseconds.
    speed: u32,
    /// Accumulator of millionths.
    acc: u32,
    /// When we last emitted a tick value.
    last_emit: Time<CLK>,
}

/// Deceleration in millionths per millisecond
const DECELERATION: u32 = 500;

#[derive(Clone, Copy)]
struct Reading<const CLK: u32>(Time<CLK>, i8);

impl<E, const CLK: u32> EncoderAccelerator<E, CLK>
where
    E: DeltaInput<CLK>,
{
    pub fn new(encoder: E) -> Self {
        EncoderAccelerator {
            encoder,
            prev: Reading(Time::new(0), 0),
            speed: 0,
            acc: 0,
            last_emit: Time::new(0),
        }
    }
}

impl<E, const CLK: u32> DeltaInput<CLK> for EncoderAccelerator<E, CLK>
where
    E: DeltaInput<CLK>,
{
    fn tick(&mut self, now: Time<CLK>) -> i8 {
        let direction = self.encoder.tick(now);

        if direction != 0 {
            let reading = Reading(now, direction);

            let dt = reading.0 - self.prev.0;

            // speed in millionths per millisecond
            let speed = if reading.1.signum() != self.prev.1.signum() {
                // direction change, however ignore if it happens too fast (due to shitty encoders).
                if dt.subsec_micros() < 4000 {
                    return 0;
                } else {
                    0
                }
            } else {
                // calculate new speed
                if dt.seconds() > 0 {
                    // too slow to impact speed
                    0
                } else if dt.subsec_millis() == 0 {
                    1200_000
                } else {
                    (1200_000 / dt.subsec_millis()) as u32
                }
            };

            if speed > self.speed || speed == 0 {
                self.speed = speed;
            }

            self.prev = reading;
            self.last_emit = now;

            // rotary movements always affect the reading. emit directly.
            return direction;
        }

        if now - self.last_emit >= Time::from_millis(1) {
            self.last_emit = now; // might not emit actually.

            if self.speed > DECELERATION {
                self.speed -= DECELERATION;
            } else {
                self.speed = 0;
            }

            // build up in the accumulator the millionths.
            self.acc += self.speed;

            if self.acc >= 1000_000 {
                self.acc = self.acc % 1000_000;

                // this will be the correct direction.
                return self.prev.1;
            }
        }

        if self.speed == 0 {
            self.acc = 0;
        }

        0
    }
}
