use crate::domain::NormalizedRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn crop_to_display_pixels(
    crop: &NormalizedRect,
    display_width: u32,
    display_height: u32,
) -> PixelRect {
    let requested_width = crop.width * f64::from(display_width);
    let requested_height = crop.height * f64::from(display_height);
    let height_candidate = even_clamped_size(requested_height, display_height);
    let width_from_height =
        even_clamped_size(f64::from(height_candidate) * 9.0 / 16.0, display_width);
    let width_candidate = even_clamped_size(requested_width, display_width);
    let height_from_width =
        even_clamped_size(f64::from(width_candidate) * 16.0 / 9.0, display_height);
    let (width, height) = if u64::from(width_from_height) * u64::from(height_candidate)
        >= u64::from(width_candidate) * u64::from(height_from_width)
    {
        (width_from_height, height_candidate)
    } else {
        (width_candidate, height_from_width)
    };
    PixelRect {
        x: even_clamped(crop.x * f64::from(display_width), display_width - width),
        y: even_clamped(crop.y * f64::from(display_height), display_height - height),
        width,
        height,
    }
}

pub fn overlay_to_canvas_pixels(position: &NormalizedRect) -> PixelRect {
    let width = even_clamped_size(position.width * 1080.0, 1080);
    let height = even_clamped_size(position.height * 1920.0, 1920);
    PixelRect {
        x: even_clamped(position.x * 1080.0, 1080 - width),
        y: even_clamped(position.y * 1920.0, 1920 - height),
        width,
        height,
    }
}

fn even_clamped(value: f64, maximum: u32) -> u32 {
    let rounded = value.round().clamp(0.0, f64::from(maximum)) as u32;
    let even = rounded - (rounded % 2);
    even.min(maximum - (maximum % 2))
}

fn even_clamped_size(value: f64, maximum: u32) -> u32 {
    let rounded = value.round().clamp(2.0, f64::from(maximum)) as u32;
    let even = rounded - (rounded % 2);
    even.clamp(2, maximum - (maximum % 2))
}

#[cfg(test)]
mod tests {
    use crate::domain::NormalizedRect;

    use super::{crop_to_display_pixels, overlay_to_canvas_pixels, PixelRect};

    #[test]
    fn maps_a_locked_crop_to_even_exact_nine_by_sixteen_pixels() {
        let crop = NormalizedRect {
            x: 0.341_797,
            y: 0.0,
            width: 0.316_406,
            height: 1.0,
        };
        let pixels = crop_to_display_pixels(&crop, 1920, 1080);
        assert_eq!(
            pixels,
            PixelRect {
                x: 656,
                y: 0,
                width: 608,
                height: 1080,
            }
        );
        assert!((f64::from(pixels.width) / f64::from(pixels.height) - 9.0 / 16.0).abs() < 0.001);
    }

    #[test]
    fn maps_overlay_coordinates_to_the_output_canvas() {
        let position = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.25,
            height: 0.1,
        };
        assert_eq!(
            overlay_to_canvas_pixels(&position),
            PixelRect {
                x: 108,
                y: 384,
                width: 270,
                height: 192,
            }
        );
    }
}
