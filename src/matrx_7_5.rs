use heapless::Vec;

const SPACE: u8 = 1;

pub fn translate<const L: usize>(s: &str, v: &mut Vec<u8, L>) {
    v.clear();

    // No UTF-8 here, we can just use bytes.
    for b in s.as_bytes() {
        if v.is_full() {
            // Don't overfill the buffer
            break;
        }

        let idx = if *b >= 48 && *b <= 57 {
            b - 48
        } else if *b >= 65 && *b <= 90 {
            // A starts after the numerics
            b - 65 + 10
        } else if *b == 32 {
            36
        } else {
            // skip undrawable char
            continue;
        };

        v.push(idx).expect("loop to break before overflow");
    }
}

pub fn render(row_index: usize, mut char_index: usize, char_offset: u8, chars: &[u8]) -> u8 {
    let mut draw_position: i8 = 0;
    let mut extra_left_shift = char_offset;

    let mut output: u8 = 0;
    const OUTPUT_BITS: u8 = 8;

    loop {
        if char_index >= chars.len() {
            break;
        }

        // If we're out of pixels to draw into.
        if draw_position >= OUTPUT_BITS as i8 {
            break;
        }

        let alpha_index = chars[char_index] as usize;
        let char = &ALPHABET[alpha_index];
        let space = if alpha_index == 36 { 2 } else { SPACE };

        let left_shift = (OUTPUT_BITS - char.width() + extra_left_shift) as i8 - draw_position;

        let row = char.rows()[row_index];
        if left_shift.abs() < 8 {
            let shifted = if left_shift >= 0 {
                row << left_shift
            } else {
                row >> left_shift.abs()
            };
            output |= shifted;
        }

        draw_position += (char.width() + space) as i8 - extra_left_shift as i8;
        char_index += 1;
        extra_left_shift = 0;
    }

    output
}

struct Char(u8, [u8; 5]);

impl Char {
    #[inline(always)]
    fn width(&self) -> u8 {
        self.0
    }

    #[inline(always)]
    fn rows(&self) -> &[u8; 5] {
        &self.1
    }
}

const ALPHABET: &[Char] = &[
    // 48-57 => 0-9
    Char(
        3, // O
        [
            0b0_0000111,
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000111,
        ],
    ),
    Char(
        2, // 1
        [
            0b0_0000001,
            0b0_0000011,
            0b0_0000001,
            0b0_0000001,
            0b0_0000001,
        ],
    ),
    Char(
        3, // 2
        [
            0b0_0000010,
            0b0_0000101,
            0b0_0000001,
            0b0_0000110,
            0b0_0000111,
        ],
    ),
    Char(
        2, // 3
        [
            0b0_0000011,
            0b0_0000001,
            0b0_0000011,
            0b0_0000001,
            0b0_0000011,
        ],
    ),
    Char(
        3, // 4
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000111,
            0b0_0000001,
            0b0_0000001,
        ],
    ),
    Char(
        3, // 5
        [
            0b0_0000111,
            0b0_0000100,
            0b0_0000110,
            0b0_0000001,
            0b0_0000110,
        ],
    ),
    Char(
        3, // 6
        [
            0b0_0000100,
            0b0_0000100,
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
        ],
    ),
    Char(
        3, // 7
        [
            0b0_0000111,
            0b0_0000001,
            0b0_0000001,
            0b0_0000010,
            0b0_0000100,
        ],
    ),
    Char(
        3, // 8
        [
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
        ],
    ),
    Char(
        3, // 9
        [
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
            0b0_0000001,
            0b0_0000011,
        ],
    ),
    // 65-90 => A-Z
    Char(
        3, // A
        [
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
            0b0_0000101,
            0b0_0000101,
        ],
    ),
    Char(
        3, // B
        [
            0b0_0000110,
            0b0_0000101,
            0b0_0000110,
            0b0_0000101,
            0b0_0000110,
        ],
    ),
    Char(
        2, // C
        [
            0b0_0000011,
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
            0b0_0000011,
        ],
    ),
    Char(
        3, // D
        [
            0b0_0000110,
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000110,
        ],
    ),
    Char(
        2, // E
        [
            0b0_0000011,
            0b0_0000010,
            0b0_0000011,
            0b0_0000010,
            0b0_0000011,
        ],
    ),
    Char(
        2, // F
        [
            0b0_0000011,
            0b0_0000010,
            0b0_0000011,
            0b0_0000010,
            0b0_0000010,
        ],
    ),
    Char(
        3, // G
        [
            0b0_0000111,
            0b0_0000100,
            0b0_0000111,
            0b0_0000101,
            0b0_0000111,
        ],
    ),
    Char(
        3, // H
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000111,
            0b0_0000101,
            0b0_0000101,
        ],
    ),
    Char(
        1, // I
        [
            0b0_0000001,
            0b0_0000000,
            0b0_0000001,
            0b0_0000001,
            0b0_0000001,
        ],
    ),
    Char(
        3, // J
        [
            0b0_0000001,
            0b0_0000001,
            0b0_0000001,
            0b0_0000101,
            0b0_0000010,
        ],
    ),
    Char(
        3, // K
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000110,
            0b0_0000101,
            0b0_0000101,
        ],
    ),
    Char(
        2, // L
        [
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
            0b0_0000011,
        ],
    ),
    Char(
        5, // M
        [
            0b0_0010001,
            0b0_0011011,
            0b0_0010101,
            0b0_0010001,
            0b0_0010001,
        ],
    ),
    Char(
        4, // N
        [
            0b0_0001001,
            0b0_0001101,
            0b0_0001011,
            0b0_0001001,
            0b0_0001001,
        ],
    ),
    Char(
        3, // O
        [
            0b0_0000010,
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000010,
        ],
    ),
    Char(
        3, // P
        [
            0b0_0000110,
            0b0_0000101,
            0b0_0000110,
            0b0_0000100,
            0b0_0000100,
        ],
    ),
    Char(
        4, // Q
        [
            0b0_0000100,
            0b0_0001010,
            0b0_0001010,
            0b0_0000110,
            0b0_0000001,
        ],
    ),
    Char(
        3, // R
        [
            0b0_0000110,
            0b0_0000101,
            0b0_0000110,
            0b0_0000101,
            0b0_0000101,
        ],
    ),
    Char(
        3, // S
        [
            0b0_0000011,
            0b0_0000100,
            0b0_0000010,
            0b0_0000001,
            0b0_0000110,
        ],
    ),
    Char(
        3, // T
        [
            0b0_0000111,
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
        ],
    ),
    Char(
        3, // U
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000111,
        ],
    ),
    Char(
        3, // V
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000101,
            0b0_0000010,
        ],
    ),
    Char(
        5, // W
        [
            0b0_0010001,
            0b0_0010001,
            0b0_0010101,
            0b0_0011011,
            0b0_0010001,
        ],
    ),
    Char(
        3, // X
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000010,
            0b0_0000101,
            0b0_0000101,
        ],
    ),
    Char(
        3, // Y
        [
            0b0_0000101,
            0b0_0000101,
            0b0_0000010,
            0b0_0000010,
            0b0_0000010,
        ],
    ),
    Char(
        3, // Z
        [
            0b0_0000111,
            0b0_0000001,
            0b0_0000010,
            0b0_0000100,
            0b0_0000111,
        ],
    ),
    Char(
        0, // <space>
        [
            0b0_0000000,
            0b0_0000000,
            0b0_0000000,
            0b0_0000000,
            0b0_0000000,
        ],
    ),
];

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn translate_simple() {
        let mut v: Vec<u8, 64> = Vec::new();
        translate("HELLO WORLD", &mut v);
        assert_eq!(&v, &[17, 14, 21, 21, 24, 36, 32, 24, 27, 21, 13]);
    }

    #[test]
    fn translate_skip_unknown() {
        let mut v: Vec<u8, 64> = Vec::new();
        translate("#!@~", &mut v);
        assert_eq!(&v, &[]);
    }

    fn test_render(s: &str, i: usize, o: u8, c: &[&str]) {
        let mut v: Vec<u8, 64> = Vec::new();
        translate(s, &mut v);

        let rows: Vec<_, 64> = (0..5)
            .map(|row| {
                let o = render(row, i, o, &v);
                let s = format!("{:#010b}|", o).replace("0b", "|").replace("0", " ");
                println!("{}", s);
                s
            })
            .collect();

        assert_eq!(&rows, c);
    }

    #[test]
    fn render_no_offset() {
        test_render(
            "ABC",
            0,
            0,
            &[
                "|111 11  |",
                "|1 1 1 1 |",
                "|111 11  |",
                "|1 1 1 1 |",
                "|1 1 11  |",
            ],
        );
    }

    #[test]
    fn render_offset_1() {
        test_render(
            "ABC",
            0,
            1,
            &[
                "|11 11  1|",
                "| 1 1 1 1|",
                "|11 11  1|",
                "| 1 1 1 1|",
                "| 1 11  1|",
            ],
        );
    }

    #[test]
    fn render_offset_2() {
        test_render(
            "ABC",
            0,
            2,
            &[
                "|1 11  11|",
                "|1 1 1 1 |",
                "|1 11  1 |",
                "|1 1 1 1 |",
                "|1 11  11|",
            ],
        );
    }

    #[test]
    fn render_offset_3() {
        test_render(
            "ABC",
            0,
            3,
            &[
                "| 11  11 |",
                "| 1 1 1  |",
                "| 11  1  |",
                "| 1 1 1  |",
                "| 11  11 |",
            ],
        );
    }

    #[test]
    fn render_offset_4() {
        test_render(
            "ABC",
            0,
            4,
            &[
                "|11  11  |",
                "|1 1 1   |",
                "|11  1   |",
                "|1 1 1   |",
                "|11  11  |",
            ],
        );
    }

    #[test]
    fn render_offset_5() {
        test_render(
            "ABC",
            0,
            5,
            &[
                "|1  11   |",
                "| 1 1    |",
                "|1  1    |",
                "| 1 1    |",
                "|1  11   |",
            ],
        );
    }

    #[test]
    fn render_offset_6() {
        test_render(
            "ABC",
            0,
            6,
            &[
                "|  11    |",
                "|1 1     |",
                "|  1     |",
                "|1 1     |",
                "|  11    |",
            ],
        );
    }

    #[test]
    fn render_offset_7() {
        test_render(
            "ABC",
            0,
            7,
            &[
                "| 11     |",
                "| 1      |",
                "| 1      |",
                "| 1      |",
                "| 11     |",
            ],
        );
    }

    #[test]
    fn render_offset_8() {
        test_render(
            "ABC",
            0,
            8,
            &[
                "|11      |",
                "|1       |",
                "|1       |",
                "|1       |",
                "|11      |",
            ],
        );
    }

    #[test]
    fn render_offset_10() {
        test_render(
            "ABC",
            0,
            10,
            &[
                "|        |",
                "|        |",
                "|        |",
                "|        |",
                "|        |",
            ],
        );
    }

    #[test]
    fn render_offset_1_0() {
        test_render(
            "ABC",
            1,
            0,
            &[
                "|11  11  |",
                "|1 1 1   |",
                "|11  1   |",
                "|1 1 1   |",
                "|11  11  |",
            ],
        );
    }

    #[test]
    fn render_offset_1_8() {
        test_render(
            "ABC",
            1,
            8,
            &[
                "|        |",
                "|        |",
                "|        |",
                "|        |",
                "|        |",
            ],
        );
    }

    #[test]
    fn render_offset_2_0() {
        test_render(
            "ABC",
            2,
            0,
            &[
                "|11      |",
                "|1       |",
                "|1       |",
                "|1       |",
                "|11      |",
            ],
        );
    }

    #[test]
    fn render_offset_3_0() {
        test_render(
            "ABC",
            3,
            0,
            &[
                "|        |",
                "|        |",
                "|        |",
                "|        |",
                "|        |",
            ],
        );
    }

    #[test]
    fn render_offset_loop() {
        let mut v: Vec<u8, 64> = Vec::new();
        translate("THIS IS RATHER GOOD", &mut v);

        for off in 0..60 {
            let _rows: Vec<_, 64> = (0..5)
                .map(|row| {
                    let o = render(row, 0, off, &v);
                    let s = format!("{:#010b}|", o).replace("0b", "|").replace("0", " ");
                    println!("{}", s);
                    s
                })
                .collect();
            println!();
        }
    }
}
