//! Hand-designed 5x7 pixel bitmap font for SAGA sprite text rendering.

pub const GLYPH_WIDTH: u32 = 5;
pub const GLYPH_HEIGHT: u32 = 7;
/// Horizontal advance in pixels including 1px spacing.
pub const GLYPH_ADVANCE: u32 = 6;

const GLYPH_ROWS: usize = GLYPH_HEIGHT as usize;
type Pattern = [&'static str; GLYPH_ROWS];

const GLYPHS: &[(char, Pattern)] = &[
    (
        '!',
        [
            "..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#..",
        ],
    ),
    (
        '"',
        [
            ".#.#.", ".#.#.", ".#.#.", ".....", ".....", ".....", ".....",
        ],
    ),
    (
        '#',
        [
            ".#.#.", ".#.#.", "#####", ".#.#.", "#####", ".#.#.", ".#.#.",
        ],
    ),
    (
        '$',
        [
            "..#..", ".####", "#.#..", ".###.", "..#.#", "####.", "..#..",
        ],
    ),
    (
        '%',
        [
            "##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##",
        ],
    ),
    (
        '&',
        [
            ".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#",
        ],
    ),
    (
        '\'',
        [
            "..#..", "..#..", ".#...", ".....", ".....", ".....", ".....",
        ],
    ),
    (
        '(',
        [
            "...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#.",
        ],
    ),
    (
        ')',
        [
            ".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#...",
        ],
    ),
    (
        '*',
        [
            ".....", ".#.#.", "..#..", "#####", "..#..", ".#.#.", ".....",
        ],
    ),
    (
        '+',
        [
            ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....",
        ],
    ),
    (
        ',',
        [
            ".....", ".....", ".....", ".....", "..#..", "..#..", ".#...",
        ],
    ),
    (
        '-',
        [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
    ),
    (
        '.',
        [
            ".....", ".....", ".....", ".....", ".....", "..#..", "..#..",
        ],
    ),
    (
        '/',
        [
            "....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#....",
        ],
    ),
    (
        '0',
        [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
    ),
    (
        '1',
        [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
    ),
    (
        '2',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
    ),
    (
        '3',
        [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        '4',
        [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
    ),
    (
        '5',
        [
            "#####", "#....", "#....", "####.", "....#", "#...#", ".###.",
        ],
    ),
    (
        '6',
        [
            ".###.", "#...#", "#....", "####.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '7',
        [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
    ),
    (
        '8',
        [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '9',
        [
            ".###.", "#...#", "#...#", ".####", "....#", "#...#", ".###.",
        ],
    ),
    (
        ':',
        [
            ".....", "..#..", "..#..", ".....", "..#..", "..#..", ".....",
        ],
    ),
    (
        ';',
        [
            ".....", "..#..", "..#..", ".....", "..#..", "..#..", ".#...",
        ],
    ),
    (
        '<',
        [
            "...#.", "..#..", ".#...", "#....", ".#...", "..#..", "...#.",
        ],
    ),
    (
        '=',
        [
            ".....", ".....", "#####", ".....", "#####", ".....", ".....",
        ],
    ),
    (
        '>',
        [
            ".#...", "..#..", "...#.", "....#", "...#.", "..#..", ".#...",
        ],
    ),
    (
        '?',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
        ],
    ),
    (
        '@',
        [
            ".###.", "#...#", "#.###", "#.#.#", "#.###", "#....", ".####",
        ],
    ),
    (
        'A',
        [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'B',
        [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
    ),
    (
        'C',
        [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
    ),
    (
        'D',
        [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
    ),
    (
        'E',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
    ),
    (
        'F',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        'G',
        [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
        ],
    ),
    (
        'H',
        [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'I',
        [
            ".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
    ),
    (
        'J',
        [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
    ),
    (
        'K',
        [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        'L',
        [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
    ),
    (
        'M',
        [
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'N',
        [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
    ),
    (
        'O',
        [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
    ),
    (
        'P',
        [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        'Q',
        [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
    ),
    (
        'R',
        [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        'S',
        [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        'T',
        [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        'U',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
    ),
    (
        'V',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
    ),
    (
        'W',
        [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
        ],
    ),
    (
        'X',
        [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
    ),
    (
        'Y',
        [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        'Z',
        [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
    ),
    (
        '[',
        [
            ".###.", ".#...", ".#...", ".#...", ".#...", ".#...", ".###.",
        ],
    ),
    (
        '\\',
        [
            "#....", ".#...", ".#...", "..#..", "...#.", "...#.", "....#",
        ],
    ),
    (
        ']',
        [
            ".###.", "...#.", "...#.", "...#.", "...#.", "...#.", ".###.",
        ],
    ),
    (
        '^',
        [
            "..#..", ".#.#.", "#...#", ".....", ".....", ".....", ".....",
        ],
    ),
    (
        '_',
        [
            ".....", ".....", ".....", ".....", ".....", ".....", "#####",
        ],
    ),
    (
        '`',
        [
            ".#...", "..#..", "...#.", ".....", ".....", ".....", ".....",
        ],
    ),
    (
        '{',
        [
            "...##", "..#..", "..#..", ".#...", "..#..", "..#..", "...##",
        ],
    ),
    (
        '|',
        [
            "..#..", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        '}',
        [
            "##...", "..#..", "..#..", "...#.", "..#..", "..#..", "##...",
        ],
    ),
    (
        '~',
        [
            ".....", ".....", ".##.#", "#..#.", ".....", ".....", ".....",
        ],
    ),
];

/// Number of glyphs in the atlas.
pub fn glyph_count() -> u32 {
    GLYPHS.len() as u32
}

/// Index of the glyph used to render `c` in the atlas, or None when the
/// character has no glyph (e.g. a space) and should only advance the pen.
pub fn glyph_index(c: char) -> Option<u32> {
    if c == ' ' {
        return None;
    }

    let glyph = canonical_glyph(c);
    GLYPHS
        .iter()
        .position(|(stored, _)| *stored == glyph)
        .map(|index| index as u32)
}

/// Rows of the glyph bitmap for `c`, most significant bit = leftmost pixel
/// (bit 4 .. bit 0). Returns None when the character has no glyph.
pub fn glyph_rows(c: char) -> Option<[u8; GLYPH_HEIGHT as usize]> {
    glyph_pattern(c).map(pattern_rows)
}

/// Builds an RGBA8 texture atlas laid out as a single horizontal strip of
/// `glyph_count()` glyphs, each GLYPH_WIDTH x GLYPH_HEIGHT pixels, with no
/// padding. Lit pixels are opaque white (255,255,255,255), unlit pixels are
/// fully transparent (0,0,0,0). Returns (width, height, rgba_bytes).
pub fn atlas_rgba() -> (u32, u32, Vec<u8>) {
    let width = glyph_count() * GLYPH_WIDTH;
    let height = GLYPH_HEIGHT;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..GLYPH_HEIGHT as usize {
        for (_, pattern) in GLYPHS {
            let row = encode_row(pattern[y]);
            for x in 0..GLYPH_WIDTH {
                if row & (1 << (GLYPH_WIDTH - 1 - x)) != 0 {
                    rgba.extend_from_slice(&[255, 255, 255, 255]);
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
    }

    (width, height, rgba)
}

/// Width in pixels of `text` when rendered at scale 1 (advance per character,
/// minus the trailing 1px spacing; 0 for empty strings).
pub fn text_width(text: &str) -> u32 {
    let count = text.chars().count() as u32;
    if count == 0 {
        0
    } else {
        count * GLYPH_ADVANCE - 1
    }
}

fn glyph_pattern(c: char) -> Option<&'static Pattern> {
    glyph_index(c).map(|index| &GLYPHS[index as usize].1)
}

fn canonical_glyph(c: char) -> char {
    if c.is_ascii_lowercase() {
        // Lowercase ASCII reuses uppercase glyph shapes to keep the atlas small.
        c.to_ascii_uppercase()
    } else if ('!'..='~').contains(&c) {
        c
    } else {
        '?'
    }
}

fn pattern_rows(pattern: &Pattern) -> [u8; GLYPH_ROWS] {
    let mut rows = [0; GLYPH_ROWS];
    for (row, line) in rows.iter_mut().zip(pattern.iter()) {
        *row = encode_row(line);
    }
    rows
}

fn encode_row(line: &str) -> u8 {
    let mut bits = 0;
    for (x, byte) in line.bytes().take(GLYPH_WIDTH as usize).enumerate() {
        if byte == b'#' {
            bits |= 1 << (GLYPH_WIDTH as usize - 1 - x);
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_dimensions_match_glyph_count() {
        let (width, height, rgba) = atlas_rgba();
        assert_eq!(width, glyph_count() * GLYPH_WIDTH);
        assert_eq!(height, GLYPH_HEIGHT);
        assert_eq!(rgba.len(), (width * height * 4) as usize);
    }

    #[test]
    fn glyph_index_is_stable_and_in_range() {
        assert_eq!(glyph_index('A'), glyph_index('A'));
        assert_eq!(glyph_index('?'), glyph_index('\u{2603}'));
        for code in 0x21_u8..=0x7e {
            let index = glyph_index(char::from(code)).expect("printable ASCII has a glyph");
            assert!(index < glyph_count());
        }
    }

    #[test]
    fn lowercase_maps_to_uppercase_rows() {
        assert_eq!(glyph_rows('a'), glyph_rows('A'));
        assert_eq!(glyph_rows('z'), glyph_rows('Z'));
    }

    #[test]
    fn space_has_no_glyph() {
        assert_eq!(glyph_index(' '), None);
        assert_eq!(glyph_rows(' '), None);
    }

    #[test]
    fn unknown_character_falls_back_to_question_mark() {
        assert_eq!(glyph_index('☃'), glyph_index('?'));
        assert_eq!(glyph_rows('☃'), glyph_rows('?'));
    }

    #[test]
    fn text_width_counts_advances_without_trailing_spacing() {
        assert_eq!(text_width(""), 0);
        assert_eq!(text_width("AB"), 11);
    }

    #[test]
    fn atlas_pixels_match_known_glyph_rows() {
        let (atlas_width, _, rgba) = atlas_rgba();
        let index = glyph_index('A').expect("A glyph");
        let rows = glyph_rows('A').expect("A rows");

        for y in 0..GLYPH_HEIGHT {
            for x in 0..GLYPH_WIDTH {
                let pixel_index = ((y * atlas_width + index * GLYPH_WIDTH + x) * 4) as usize;
                let expected = if rows[y as usize] & (1 << (GLYPH_WIDTH - 1 - x)) != 0 {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 0]
                };
                assert_eq!(rgba[pixel_index..pixel_index + 4], expected);
            }
        }
    }
}
