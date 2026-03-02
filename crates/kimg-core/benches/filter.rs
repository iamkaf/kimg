use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::filter::{apply_hsl_filter, gradient_map, invert, levels, posterize, HslFilterConfig};
use kimg_core::pixel::Rgba;

fn solid_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(180, 90, 60, 220));
    buf
}

fn bench_apply_hsl_filter(c: &mut Criterion) {
    let mut cfg = HslFilterConfig::default();
    cfg.hue_deg = 30.0;
    cfg.saturation = 0.2;
    cfg.lightness = -0.1;
    cfg.brightness = 0.1;
    cfg.contrast = 0.15;
    cfg.temperature = 0.1;

    c.bench_function("apply_hsl_filter/512", |b| {
        b.iter_batched(
            || solid_buf(512),
            |mut buf| {
                apply_hsl_filter(black_box(&mut buf), black_box(&cfg));
                black_box(buf)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_invert(c: &mut Criterion) {
    c.bench_function("invert/512", |b| {
        b.iter_batched(
            || solid_buf(512),
            |mut buf| {
                invert(black_box(&mut buf));
                black_box(buf)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_levels(c: &mut Criterion) {
    c.bench_function("levels/512", |b| {
        b.iter_batched(
            || solid_buf(512),
            |mut buf| {
                levels(black_box(&mut buf), 10, 240, 1.0, 0, 255);
                black_box(buf)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_posterize(c: &mut Criterion) {
    c.bench_function("posterize/512", |b| {
        b.iter_batched(
            || solid_buf(512),
            |mut buf| {
                posterize(black_box(&mut buf), 4);
                black_box(buf)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_gradient_map(c: &mut Criterion) {
    let stops = vec![
        (0.0, Rgba::new(0, 0, 128, 255)),
        (1.0, Rgba::new(255, 200, 0, 255)),
    ];
    c.bench_function("gradient_map/512", |b| {
        b.iter_batched(
            || solid_buf(512),
            |mut buf| {
                gradient_map(black_box(&mut buf), black_box(&stops));
                black_box(buf)
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(
    benches,
    bench_apply_hsl_filter,
    bench_invert,
    bench_levels,
    bench_posterize,
    bench_gradient_map
);
criterion_main!(benches);
