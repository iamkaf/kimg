use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::codec::{decode_jpeg, decode_png, decode_webp, encode_jpeg, encode_png, encode_webp};
use kimg_core::pixel::Rgba;

fn solid_buf_512() -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(512, 512);
    buf.fill(Rgba::new(180, 90, 60, 255));
    buf
}

fn bench_encode_png(c: &mut Criterion) {
    let buf = solid_buf_512();
    c.bench_function("encode_png/512", |b| {
        b.iter(|| black_box(encode_png(black_box(&buf)).unwrap()))
    });
}

fn bench_decode_png(c: &mut Criterion) {
    let buf = solid_buf_512();
    let encoded = encode_png(&buf).unwrap();
    c.bench_function("decode_png/512", |b| {
        b.iter(|| black_box(decode_png(black_box(&encoded)).unwrap()))
    });
}

fn bench_encode_jpeg(c: &mut Criterion) {
    let buf = solid_buf_512();
    c.bench_function("encode_jpeg/512", |b| {
        b.iter(|| black_box(encode_jpeg(black_box(&buf), 85).unwrap()))
    });
}

fn bench_decode_jpeg(c: &mut Criterion) {
    let buf = solid_buf_512();
    let encoded = encode_jpeg(&buf, 85).unwrap();
    c.bench_function("decode_jpeg/512", |b| {
        b.iter(|| black_box(decode_jpeg(black_box(&encoded)).unwrap()))
    });
}

fn bench_encode_webp(c: &mut Criterion) {
    let buf = solid_buf_512();
    c.bench_function("encode_webp/512", |b| {
        b.iter(|| black_box(encode_webp(black_box(&buf)).unwrap()))
    });
}

fn bench_decode_webp(c: &mut Criterion) {
    let buf = solid_buf_512();
    let encoded = encode_webp(&buf).unwrap();
    c.bench_function("decode_webp/512", |b| {
        b.iter(|| black_box(decode_webp(black_box(&encoded)).unwrap()))
    });
}

criterion_group!(
    benches,
    bench_encode_png,
    bench_decode_png,
    bench_encode_jpeg,
    bench_decode_jpeg,
    bench_encode_webp,
    bench_decode_webp
);
criterion_main!(benches);
