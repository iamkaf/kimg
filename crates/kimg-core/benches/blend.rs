use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kimg_core::blend::{blend, blend_normal, BlendMode};
use kimg_core::buffer::ImageBuffer;
use kimg_core::pixel::Rgba;

fn make_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(128, 64, 192, 200));
    buf
}

fn bench_blend_normal(c: &mut Criterion) {
    let mut group = c.benchmark_group("blend_normal");
    for size in [64u32, 512, 2048] {
        let src = make_buf(size);
        group.bench_function(size.to_string(), |b| {
            b.iter_batched(
                || make_buf(size),
                |mut dst| {
                    blend_normal(black_box(&mut dst), black_box(&src));
                    black_box(dst)
                },
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

fn bench_blend_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("blend_mode");
    let src = make_buf(512);
    for mode in [BlendMode::Multiply, BlendMode::Screen, BlendMode::Overlay] {
        group.bench_function(mode.as_str(), |b| {
            b.iter_batched(
                || make_buf(512),
                |mut dst| {
                    blend(black_box(&mut dst), black_box(&src), black_box(mode));
                    black_box(dst)
                },
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_blend_normal, bench_blend_mode);
criterion_main!(benches);
