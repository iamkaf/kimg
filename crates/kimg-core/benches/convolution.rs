use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::convolution::{box_blur, convolve, gaussian_blur, Kernel};
use kimg_core::pixel::Rgba;

fn solid_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(150, 100, 200, 255));
    buf
}

fn bench_convolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("convolve");
    let src = solid_buf(512);

    let k3 = Kernel::box_blur_3x3();
    group.bench_function("3x3/512", |b| {
        b.iter(|| black_box(convolve(black_box(&src), black_box(&k3))))
    });

    let k5 = Kernel::box_blur_5x5();
    group.bench_function("5x5/512", |b| {
        b.iter(|| black_box(convolve(black_box(&src), black_box(&k5))))
    });

    group.finish();
}

fn bench_box_blur(c: &mut Criterion) {
    let mut group = c.benchmark_group("box_blur");
    let src = solid_buf(512);

    group.bench_function("r1/512", |b| {
        b.iter(|| black_box(box_blur(black_box(&src), black_box(1u32))))
    });

    group.bench_function("r3/512", |b| {
        b.iter(|| black_box(box_blur(black_box(&src), black_box(3u32))))
    });

    group.finish();
}

fn bench_gaussian_blur(c: &mut Criterion) {
    let mut group = c.benchmark_group("gaussian_blur");
    let src = solid_buf(512);

    group.bench_function("r1/512", |b| {
        b.iter(|| black_box(gaussian_blur(black_box(&src), black_box(1u32))))
    });

    group.bench_function("r2/512", |b| {
        b.iter(|| black_box(gaussian_blur(black_box(&src), black_box(2u32))))
    });

    group.finish();
}

criterion_group!(benches, bench_convolve, bench_box_blur, bench_gaussian_blur);
criterion_main!(benches);
