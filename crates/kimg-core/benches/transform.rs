use criterion::{black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode};
use kimg_core::buffer::ImageBuffer;
use kimg_core::pixel::Rgba;
use kimg_core::transform::{
    crop, resize_bilinear, resize_lanczos3, resize_nearest, rotate_bilinear, trim_alpha,
};
use std::time::Duration;

fn solid_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(200, 100, 50, 255));
    buf
}

fn configure_resize_group(group: &mut BenchmarkGroup<'_, WallTime>, in_size: u32, quality: &str) {
    match (quality, in_size) {
        ("nearest", 2048) => {
            group.sample_size(20);
            group.sampling_mode(SamplingMode::Flat);
            group.measurement_time(Duration::from_secs(8));
            group.warm_up_time(Duration::from_secs(1));
        }
        ("bilinear", 2048) => {
            group.sample_size(10);
            group.sampling_mode(SamplingMode::Flat);
            group.measurement_time(Duration::from_secs(8));
            group.warm_up_time(Duration::from_secs(1));
        }
        ("lanczos3", 512) => {
            group.sample_size(20);
            group.sampling_mode(SamplingMode::Flat);
            group.measurement_time(Duration::from_secs(8));
            group.warm_up_time(Duration::from_secs(1));
        }
        ("lanczos3", 2048) => {
            group.sample_size(10);
            group.sampling_mode(SamplingMode::Flat);
            group.measurement_time(Duration::from_secs(8));
            group.warm_up_time(Duration::from_secs(1));
        }
        _ => {
            group.sample_size(100);
            group.sampling_mode(SamplingMode::Auto);
            group.measurement_time(Duration::from_secs(5));
            group.warm_up_time(Duration::from_secs(3));
        }
    }
}

fn bench_resize_nearest(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_nearest");
    for (in_size, out_size) in [(64u32, 128u32), (512, 1024), (2048, 4096)] {
        configure_resize_group(&mut group, in_size, "nearest");
        let src = solid_buf(in_size);
        group.bench_function(format!("{in_size}→{out_size}"), |b| {
            b.iter(|| black_box(resize_nearest(black_box(&src), out_size, out_size)))
        });
    }
    group.finish();
}

fn bench_resize_bilinear(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_bilinear");
    for (in_size, out_size) in [(64u32, 128u32), (512, 1024), (2048, 4096)] {
        configure_resize_group(&mut group, in_size, "bilinear");
        let src = solid_buf(in_size);
        group.bench_function(format!("{in_size}→{out_size}"), |b| {
            b.iter(|| black_box(resize_bilinear(black_box(&src), out_size, out_size)))
        });
    }
    group.finish();
}

fn bench_resize_lanczos3(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_lanczos3");
    for (in_size, out_size) in [(64u32, 128u32), (512, 1024), (2048, 4096)] {
        configure_resize_group(&mut group, in_size, "lanczos3");
        let src = solid_buf(in_size);
        group.bench_function(format!("{in_size}→{out_size}"), |b| {
            b.iter(|| black_box(resize_lanczos3(black_box(&src), out_size, out_size)))
        });
    }
    group.finish();
}

fn bench_crop(c: &mut Criterion) {
    let src = solid_buf(512);
    c.bench_function("crop/512", |b| {
        b.iter(|| black_box(crop(black_box(&src), 0, 0, 256, 256)))
    });
}

fn bench_trim_alpha(c: &mut Criterion) {
    // Use a half-transparent image so trim finds a real bounding box.
    let mut src = ImageBuffer::new_transparent(512, 512);
    for y in 128..384u32 {
        for x in 128..384u32 {
            let i = (y * 512 + x) as usize * 4;
            src.data[i] = 200;
            src.data[i + 1] = 100;
            src.data[i + 2] = 50;
            src.data[i + 3] = 255;
        }
    }
    c.bench_function("trim_alpha/512", |b| {
        b.iter(|| black_box(trim_alpha(black_box(&src))))
    });
}

fn bench_rotate_bilinear(c: &mut Criterion) {
    let mut group = c.benchmark_group("rotate_bilinear");
    let src = solid_buf(512);
    for angle in [90.0f64, 45.0] {
        group.bench_function(format!("{angle}°/512"), |b| {
            b.iter(|| black_box(rotate_bilinear(black_box(&src), black_box(angle))))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_resize_nearest,
    bench_resize_bilinear,
    bench_resize_lanczos3,
    bench_crop,
    bench_trim_alpha,
    bench_rotate_bilinear
);
criterion_main!(benches);
