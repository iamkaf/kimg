//! Shape rasterization utilities.
//!
//! Shape layers store primitive parameters and are rasterized to an
//! [`ImageBuffer`](crate::buffer::ImageBuffer) during render/export.

use crate::buffer::ImageBuffer;
use crate::layer::{ShapeLayerData, ShapePoint, ShapeType};
use crate::pixel::Rgba;

/// Rasterize a shape layer into its local RGBA buffer.
pub fn render_shape(shape: &ShapeLayerData) -> ImageBuffer {
    let width = shape.width.max(1);
    let height = shape.height.max(1);
    let mut buffer = ImageBuffer::new_transparent(width, height);

    for y in 0..height {
        for x in 0..width {
            let sample_x = x as f64 + 0.5;
            let sample_y = y as f64 + 0.5;

            let pixel = match shape.shape_type {
                ShapeType::Rectangle => sample_rect(shape, sample_x, sample_y),
                ShapeType::RoundedRect => sample_rounded_rect(shape, sample_x, sample_y),
                ShapeType::Ellipse => sample_ellipse(shape, sample_x, sample_y),
                ShapeType::Line => sample_line(shape, sample_x, sample_y),
                ShapeType::Polygon => sample_polygon(shape, sample_x, sample_y),
            };

            if let Some(pixel) = pixel {
                buffer.set_pixel(x, y, pixel);
            }
        }
    }

    buffer
}

fn sample_rect(shape: &ShapeLayerData, sample_x: f64, sample_y: f64) -> Option<Rgba> {
    let width = shape.width.max(1) as f64;
    let height = shape.height.max(1) as f64;
    if !contains_rect(sample_x, sample_y, width, height) {
        return None;
    }

    stroke_or_fill(
        shape,
        true,
        shape
            .stroke
            .is_some_and(|stroke| is_rect_stroke(sample_x, sample_y, width, height, stroke.width)),
    )
}

fn sample_rounded_rect(shape: &ShapeLayerData, sample_x: f64, sample_y: f64) -> Option<Rgba> {
    let width = shape.width.max(1) as f64;
    let height = shape.height.max(1) as f64;
    let radius = shape.radius.min(shape.width / 2).min(shape.height / 2) as f64;
    let outer = contains_rounded_rect(sample_x, sample_y, width, height, radius);
    if !outer {
        return None;
    }

    let stroke = shape.stroke.is_some_and(|stroke| {
        let stroke_width = stroke.width as f64;
        let inner_width = width - stroke_width * 2.0;
        let inner_height = height - stroke_width * 2.0;
        if inner_width <= 0.0 || inner_height <= 0.0 {
            return true;
        }

        !contains_rounded_rect(
            sample_x - stroke_width,
            sample_y - stroke_width,
            inner_width,
            inner_height,
            (radius - stroke_width).max(0.0),
        )
    });

    stroke_or_fill(shape, true, stroke)
}

fn sample_ellipse(shape: &ShapeLayerData, sample_x: f64, sample_y: f64) -> Option<Rgba> {
    let width = shape.width.max(1) as f64;
    let height = shape.height.max(1) as f64;
    let outer = contains_ellipse(sample_x, sample_y, width, height);
    if !outer {
        return None;
    }

    let stroke = shape.stroke.is_some_and(|stroke| {
        let stroke_width = stroke.width as f64;
        let inner_width = width - stroke_width * 2.0;
        let inner_height = height - stroke_width * 2.0;
        if inner_width <= 0.0 || inner_height <= 0.0 {
            return true;
        }

        !contains_ellipse(
            sample_x - stroke_width,
            sample_y - stroke_width,
            inner_width,
            inner_height,
        )
    });

    stroke_or_fill(shape, true, stroke)
}

fn sample_line(shape: &ShapeLayerData, sample_x: f64, sample_y: f64) -> Option<Rgba> {
    let width = shape.width.max(1) as f64;
    let height = shape.height.max(1) as f64;
    let color = shape.stroke.map(|stroke| stroke.color).or(shape.fill)?;
    let line_width = shape
        .stroke
        .map(|stroke| stroke.width.max(1) as f64)
        .unwrap_or(1.0);
    let distance = distance_to_segment(
        sample_x,
        sample_y,
        0.5,
        0.5,
        (width - 0.5).max(0.5),
        (height - 0.5).max(0.5),
    );
    if distance <= line_width / 2.0 {
        Some(color)
    } else {
        None
    }
}

fn sample_polygon(shape: &ShapeLayerData, sample_x: f64, sample_y: f64) -> Option<Rgba> {
    if shape.points.len() < 3 {
        return None;
    }

    let fill = point_in_polygon(&shape.points, sample_x, sample_y);
    let stroke = shape.stroke.is_some_and(|stroke| {
        distance_to_polygon_edges(&shape.points, sample_x, sample_y)
            <= stroke.width.max(1) as f64 / 2.0
    });

    if !fill && !stroke {
        return None;
    }

    stroke_or_fill(shape, fill, stroke)
}

fn stroke_or_fill(shape: &ShapeLayerData, fill_hit: bool, stroke_hit: bool) -> Option<Rgba> {
    if stroke_hit {
        if let Some(stroke) = shape.stroke {
            return Some(stroke.color);
        }
    }

    if fill_hit {
        return shape.fill;
    }

    None
}

fn contains_rect(sample_x: f64, sample_y: f64, width: f64, height: f64) -> bool {
    sample_x >= 0.0 && sample_x <= width && sample_y >= 0.0 && sample_y <= height
}

fn is_rect_stroke(
    sample_x: f64,
    sample_y: f64,
    width: f64,
    height: f64,
    stroke_width: u32,
) -> bool {
    let stroke_width = stroke_width.max(1) as f64;
    sample_x <= stroke_width
        || sample_x >= width - stroke_width
        || sample_y <= stroke_width
        || sample_y >= height - stroke_width
}

fn contains_rounded_rect(
    sample_x: f64,
    sample_y: f64,
    width: f64,
    height: f64,
    radius: f64,
) -> bool {
    if width <= 0.0 || height <= 0.0 {
        return false;
    }

    if radius <= 0.0 {
        return contains_rect(sample_x, sample_y, width, height);
    }

    let clamped_x = sample_x.clamp(radius, width - radius);
    let clamped_y = sample_y.clamp(radius, height - radius);
    let dx = sample_x - clamped_x;
    let dy = sample_y - clamped_y;
    dx * dx + dy * dy <= radius * radius
}

fn contains_ellipse(sample_x: f64, sample_y: f64, width: f64, height: f64) -> bool {
    if width <= 0.0 || height <= 0.0 {
        return false;
    }

    let radius_x = width / 2.0;
    let radius_y = height / 2.0;
    let dx = sample_x - radius_x;
    let dy = sample_y - radius_y;
    (dx * dx) / (radius_x * radius_x) + (dy * dy) / (radius_y * radius_y) <= 1.0
}

fn point_in_polygon(points: &[ShapePoint], sample_x: f64, sample_y: f64) -> bool {
    let mut inside = false;
    let mut previous = points.last().copied().unwrap_or(ShapePoint::new(0, 0));

    for current in points {
        let x1 = previous.x as f64;
        let y1 = previous.y as f64;
        let x2 = current.x as f64;
        let y2 = current.y as f64;

        let intersects = ((y1 > sample_y) != (y2 > sample_y))
            && (sample_x < (x2 - x1) * (sample_y - y1) / (y2 - y1) + x1);
        if intersects {
            inside = !inside;
        }

        previous = *current;
    }

    inside
}

fn distance_to_polygon_edges(points: &[ShapePoint], sample_x: f64, sample_y: f64) -> f64 {
    let mut best = f64::INFINITY;
    let mut previous = points.last().copied().unwrap_or(ShapePoint::new(0, 0));

    for current in points {
        let distance = distance_to_segment(
            sample_x,
            sample_y,
            previous.x as f64,
            previous.y as f64,
            current.x as f64,
            current.y as f64,
        );
        best = best.min(distance);
        previous = *current;
    }

    best
}

fn distance_to_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab_len_sq = abx * abx + aby * aby;

    if ab_len_sq <= f64::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }

    let t = ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0);
    let closest_x = ax + abx * t;
    let closest_y = ay + aby * t;
    ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{ShapeLayerData, ShapeStroke};

    #[test]
    fn rectangle_fill_and_stroke_render() {
        let shape = ShapeLayerData::new(
            ShapeType::Rectangle,
            4,
            4,
            0,
            Some(Rgba::new(255, 0, 0, 255)),
            Some(ShapeStroke::new(Rgba::new(255, 255, 255, 255), 1)),
            Vec::new(),
        );

        let buf = render_shape(&shape);
        assert_eq!(buf.get_pixel(0, 0), Rgba::new(255, 255, 255, 255));
        assert_eq!(buf.get_pixel(1, 1), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn ellipse_renders_transparent_corners() {
        let shape = ShapeLayerData::new(
            ShapeType::Ellipse,
            5,
            5,
            0,
            Some(Rgba::new(0, 255, 0, 255)),
            None,
            Vec::new(),
        );

        let buf = render_shape(&shape);
        assert_eq!(buf.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert_eq!(buf.get_pixel(2, 2), Rgba::new(0, 255, 0, 255));
    }

    #[test]
    fn polygon_fill_renders_interior() {
        let shape = ShapeLayerData::new(
            ShapeType::Polygon,
            5,
            5,
            0,
            Some(Rgba::new(0, 0, 255, 255)),
            None,
            vec![
                ShapePoint::new(0, 0),
                ShapePoint::new(4, 0),
                ShapePoint::new(2, 4),
            ],
        );

        let buf = render_shape(&shape);
        assert_eq!(buf.get_pixel(2, 2), Rgba::new(0, 0, 255, 255));
    }
}
