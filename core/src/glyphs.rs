pub(crate) const GLYPH_WIDTH: u32 = 5;
pub(crate) const GLYPH_HEIGHT: u32 = 7;

const BLANK: [u8; 7] = [0; 7];

const DIGITS: [[u8; 7]; 10] = [
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    [
        0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
    ],
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ],
];

const TIMES: [u8; 7] = [
    0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001,
];

/// Rows of the glyph for `c`, one byte per row with the leftmost column in bit 4.
/// Characters outside the dimension label alphabet render blank.
pub(crate) fn glyph(c: char) -> &'static [u8; 7] {
    match c {
        '0'..='9' => &DIGITS[c as usize - '0' as usize],
        '×' => &TIMES,
        _ => &BLANK,
    }
}
