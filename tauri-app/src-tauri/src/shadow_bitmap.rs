#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ShadowShape {
    pub width: usize,
    pub height: usize,
    pub padding: usize,
    pub corner: f32,
    pub blur: f32,
    pub offset_y: f32,
}

fn coverage(x: f32, y: f32, width: f32, height: f32, corner: f32) -> f32 {
    let radius = corner.min(width / 2.0).min(height / 2.0);
    let dx = (x - width / 2.0).abs() - (width / 2.0 - radius);
    let dy = (y - height / 2.0).abs() - (height / 2.0 - radius);
    let distance = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;
    (0.5 - distance).clamp(0.0, 1.0)
}

fn box_blur(
    input: &[f32],
    width: usize,
    height: usize,
    radius: usize,
    horizontal: bool,
) -> Vec<f32> {
    let mut result = vec![0.0; input.len()];
    let (lines, length) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let index = |line: usize, position: usize| {
        if horizontal {
            line * width + position
        } else {
            position * width + line
        }
    };
    let denominator = (radius * 2 + 1) as f32;
    for line in 0..lines {
        let mut sum: f32 = (0..radius.min(length.saturating_sub(1)))
            .map(|position| input[index(line, position)])
            .sum();
        for position in 0..length {
            if position + radius < length {
                sum += input[index(line, position + radius)];
            }
            if position > radius {
                sum -= input[index(line, position - radius - 1)];
            }
            result[index(line, position)] = sum / denominator;
        }
    }
    result
}

pub(crate) fn render(shape: ShadowShape) -> Vec<u8> {
    let width = shape.width + 2 * shape.padding;
    let height = shape.height + 2 * shape.padding;
    let mut mask = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            mask[y * width + x] = coverage(
                x as f32 + 0.5 - shape.padding as f32,
                y as f32 + 0.5 - shape.padding as f32 - shape.offset_y,
                shape.width as f32,
                shape.height as f32,
                shape.corner,
            );
        }
    }
    // Three sliding box filters approximate a Gaussian without work proportional to blur radius.
    let sigma = shape.blur / 3.0;
    let ideal = (4.0 * sigma * sigma + 1.0).sqrt();
    let mut lower = ideal.floor() as usize;
    if lower % 2 == 0 {
        lower = lower.saturating_sub(1);
    }
    lower = lower.max(1);
    let upper = lower + 2;
    let low_count =
        ((12.0 * sigma * sigma - 3.0 * (lower * lower) as f32 - 12.0 * lower as f32 - 9.0)
            / (-4.0 * lower as f32 - 4.0))
            .round()
            .clamp(0.0, 3.0) as usize;
    for pass in 0..3 {
        let radius = (if pass < low_count { lower } else { upper }) / 2;
        if radius > 0 {
            mask = box_blur(&mask, width, height, radius, true);
            mask = box_blur(&mask, width, height, radius, false);
        }
    }
    let mut pixels = vec![0; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let panel = coverage(
                x as f32 + 0.5 - shape.padding as f32,
                y as f32 + 0.5 - shape.padding as f32,
                shape.width as f32,
                shape.height as f32,
                shape.corner,
            );
            // Keep the glass interior clear: the backdrop must not sample its own black shadow.
            pixels[(y * width + x) * 4 + 3] =
                (255.0 * mask[y * width + x] * (1.0 - panel)).round() as u8;
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_is_hollow_and_extends_outside_each_panel_edge() {
        let shape = ShadowShape {
            width: 80,
            height: 60,
            padding: 16,
            corner: 8.0,
            blur: 12.0,
            offset_y: 0.0,
        };
        let pixels = render(shape);
        let width = shape.width + 2 * shape.padding;
        let alpha = |x, y| pixels[(y * width + x) * 4 + 3];
        assert_eq!(alpha(56, 46), 0);
        for (x, y) in [(15, 46), (96, 46), (56, 15), (56, 76)] {
            assert!(alpha(x, y) > 0);
        }
        assert_eq!(alpha(0, 0), 0);
    }
}
