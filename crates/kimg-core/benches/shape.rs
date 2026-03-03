use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::layer::{ShapeLayerData, ShapePoint, ShapeStroke, ShapeType};
use kimg_core::pixel::Rgba;
use kimg_core::shape::render_shape;

fn rectangle_shape(size: u32) -> ShapeLayerData {
    ShapeLayerData::new(
        ShapeType::Rectangle,
        size,
        size,
        0,
        Some(Rgba::new(220, 80, 120, 255)),
        Some(ShapeStroke::new(Rgba::new(255, 255, 255, 255), 4)),
        Vec::new(),
    )
}

fn polygon_shape(size: u32) -> ShapeLayerData {
    let edge = size as i32 - 1;
    ShapeLayerData::new(
        ShapeType::Polygon,
        size,
        size,
        0,
        Some(Rgba::new(70, 150, 255, 255)),
        Some(ShapeStroke::new(Rgba::new(255, 245, 210, 255), 3)),
        vec![
            ShapePoint::new(size as i32 / 2, 12),
            ShapePoint::new(edge - 70, edge / 3),
            ShapePoint::new(edge - 24, edge / 3),
            ShapePoint::new(edge - 96, edge / 2),
            ShapePoint::new(edge - 24, edge - 48),
            ShapePoint::new(size as i32 / 2, edge - 96),
            ShapePoint::new(24, edge - 48),
            ShapePoint::new(96, edge / 2),
            ShapePoint::new(24, edge / 3),
            ShapePoint::new(70, edge / 3),
        ],
    )
}

fn bench_shape_rasterize_rectangle(c: &mut Criterion) {
    let shape = rectangle_shape(512);
    c.bench_function("shape/rasterize_rectangle/512", |b| {
        b.iter(|| black_box(render_shape(black_box(&shape))))
    });
}

fn bench_shape_rasterize_polygon(c: &mut Criterion) {
    let shape = polygon_shape(512);
    c.bench_function("shape/rasterize_polygon/512", |b| {
        b.iter(|| black_box(render_shape(black_box(&shape))))
    });
}

criterion_group!(
    benches,
    bench_shape_rasterize_rectangle,
    bench_shape_rasterize_polygon
);
criterion_main!(benches);
