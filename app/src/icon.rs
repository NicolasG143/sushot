const BACKGROUND: [u8; 4] = [0x2D, 0x7F, 0xF9, 0xFF];
const FOREGROUND: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// RGBA pixels of the tray icon: a rounded square with viewfinder corners. The flash
/// variant swaps the two colors so a copy is visible at a glance.
pub(crate) fn rgba(size: u32, flash: bool) -> Vec<u8> {
    let (background, foreground) = if flash {
        (FOREGROUND, BACKGROUND)
    } else {
        (BACKGROUND, FOREGROUND)
    };
    let mut pixels = Vec::with_capacity(size as usize * size as usize * 4);
    for y in 0..size {
        for x in 0..size {
            let color = if !inside_rounded_square(x, y, size) {
                CLEAR
            } else if on_viewfinder(x, y, size) {
                foreground
            } else {
                background
            };
            pixels.extend_from_slice(&color);
        }
    }
    pixels
}

fn inside_rounded_square(x: u32, y: u32, size: u32) -> bool {
    let radius = size / 5;
    let nearest = |value: u32| value.clamp(radius, size - 1 - radius);
    let (dx, dy) = (
        i64::from(x) - i64::from(nearest(x)),
        i64::from(y) - i64::from(nearest(y)),
    );
    dx * dx + dy * dy <= i64::from(radius) * i64::from(radius)
}

fn on_viewfinder(x: u32, y: u32, size: u32) -> bool {
    let inset = size / 4;
    let reach = size / 4;
    let thickness = (size / 10).max(1);
    let far = size - 1 - inset;
    let near_edge = |value: u32| (inset..inset + thickness).contains(&value);
    let far_edge = |value: u32| (far + 1 - thickness..=far).contains(&value);
    let near_run = |value: u32| (inset..inset + reach).contains(&value);
    let far_run = |value: u32| (far + 1 - reach..=far).contains(&value);

    let horizontal = (near_edge(y) || far_edge(y)) && (near_run(x) || far_run(x));
    let vertical = (near_edge(x) || far_edge(x)) && (near_run(y) || far_run(y));
    horizontal || vertical
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * size + x) * 4) as usize;
        pixels[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn icon_holds_one_rgba_pixel_per_position() {
        assert_eq!(rgba(32, false).len(), 32 * 32 * 4);
    }

    #[test]
    fn corners_are_transparent_and_the_body_is_opaque() {
        let pixels = rgba(32, false);

        assert_eq!(pixel(&pixels, 32, 0, 0), CLEAR);
        assert_eq!(pixel(&pixels, 32, 31, 31), CLEAR);
        assert_eq!(pixel(&pixels, 32, 16, 16), BACKGROUND);
        assert_eq!(pixel(&pixels, 32, 2, 16), BACKGROUND);
    }

    #[test]
    fn viewfinder_corner_uses_the_foreground_color() {
        let pixels = rgba(32, false);

        assert_eq!(pixel(&pixels, 32, 8, 8), FOREGROUND);
        assert_eq!(pixel(&pixels, 32, 23, 23), FOREGROUND);
    }

    #[test]
    fn flash_variant_swaps_the_colors_and_keeps_the_shape() {
        let normal = rgba(32, false);
        let flash = rgba(32, true);

        assert_eq!(pixel(&flash, 32, 16, 16), FOREGROUND);
        assert_eq!(pixel(&flash, 32, 8, 8), BACKGROUND);
        assert_eq!(pixel(&flash, 32, 0, 0), CLEAR);
        let shape = |pixels: &[u8]| {
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3] == 0)
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(&normal), shape(&flash));
    }
}
