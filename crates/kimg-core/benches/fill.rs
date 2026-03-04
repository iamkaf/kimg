use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::fill::bucket_fill;
use kimg_core::pixel::Rgba;

fn contiguous_region_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(100, 100, 100, 255));
    for y in size / 4..(size / 4) * 3 {
        for x in size / 4..(size / 4) * 3 {
            buf.set_pixel(x, y, Rgba::new(180, 60, 60, 255));
        }
    }
    buf
}

fn repeated_color_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    for y in 0..size {
        for x in 0..size {
            let pixel = if (x + y) % 2 == 0 {
                Rgba::new(100, 90, 120, 255)
            } else {
                Rgba::new(20, 20, 20, 255)
            };
            buf.set_pixel(x, y, pixel);
        }
    }
    buf
}

fn noisy_tolerance_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    for y in 0..size {
        for x in 0..size {
            let jitter = ((x * 17 + y * 31) % 10) as u8;
            buf.set_pixel(
                x,
                y,
                Rgba::new(120 + jitter, 110 + jitter, 100 + jitter, 128 + jitter),
            );
        }
    }
    buf
}

fn bench_bucket_fill_contiguous(c: &mut Criterion) {
    let src = contiguous_region_buf(512);
    c.bench_function("bucket_fill/contiguous/512", |b| {
        b.iter_batched(
            || src.clone(),
            |mut buf| {
                black_box(bucket_fill(
                    &mut buf,
                    256,
                    256,
                    Rgba::new(0, 255, 0, 255),
                    true,
                    0,
                    false,
                ))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_bucket_fill_non_contiguous(c: &mut Criterion) {
    let src = repeated_color_buf(512);
    c.bench_function("bucket_fill/non_contiguous/512", |b| {
        b.iter_batched(
            || src.clone(),
            |mut buf| {
                black_box(bucket_fill(
                    &mut buf,
                    0,
                    0,
                    Rgba::new(255, 0, 0, 255),
                    false,
                    0,
                    false,
                ))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_bucket_fill_tolerance(c: &mut Criterion) {
    let src = noisy_tolerance_buf(512);
    c.bench_function("bucket_fill/tolerance/512", |b| {
        b.iter_batched(
            || src.clone(),
            |mut buf| {
                black_box(bucket_fill(
                    &mut buf,
                    0,
                    0,
                    Rgba::new(0, 0, 255, 255),
                    false,
                    12,
                    false,
                ))
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_bucket_fill_contiguous,
    bench_bucket_fill_non_contiguous,
    bench_bucket_fill_tolerance
);
criterion_main!(benches);
