use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::codec::{
    decode_jpeg, decode_png, decode_webp, encode_jpeg, encode_png, encode_webp,
};

/// Build a deterministic textured image so codec benches don't measure
/// unrealistic best-case compression on a flat solid fill.
fn textured_buf_512() -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(512, 512);
    for y in 0..512u32 {
        for x in 0..512u32 {
            let i = (y * 512 + x) as usize * 4;

            let base_r = ((x * 255) / 511) as u8;
            let base_g = ((y * 255) / 511) as u8;
            let base_b = (((x ^ y) * 255) / 511) as u8;

            let checker = if ((x / 16) + (y / 16)) % 2 == 0 {
                28
            } else {
                -28
            };
            let ripple = ((((x * x + y * 3) % 97) as i32) - 48) / 2;

            let r = (base_r as i32 + checker + ripple).clamp(0, 255) as u8;
            let g = (base_g as i32 - checker / 2 + ripple).clamp(0, 255) as u8;
            let b = (base_b as i32 + checker / 3 - ripple).clamp(0, 255) as u8;
            let a = if ((x / 32) + (y / 32)) % 3 == 0 {
                255
            } else {
                224
            };

            buf.data[i] = r;
            buf.data[i + 1] = g;
            buf.data[i + 2] = b;
            buf.data[i + 3] = a;
        }
    }

    buf
}

fn bench_encode_png(c: &mut Criterion) {
    let buf = textured_buf_512();
    c.bench_function("encode_png/512", |b| {
        b.iter(|| black_box(encode_png(black_box(&buf)).unwrap()))
    });
}

fn bench_decode_png(c: &mut Criterion) {
    let buf = textured_buf_512();
    let encoded = encode_png(&buf).unwrap();
    c.bench_function("decode_png/512", |b| {
        b.iter(|| black_box(decode_png(black_box(&encoded)).unwrap()))
    });
}

fn bench_encode_jpeg(c: &mut Criterion) {
    let buf = textured_buf_512();
    c.bench_function("encode_jpeg/512", |b| {
        b.iter(|| black_box(encode_jpeg(black_box(&buf), 85).unwrap()))
    });
}

fn bench_decode_jpeg(c: &mut Criterion) {
    let buf = textured_buf_512();
    let encoded = encode_jpeg(&buf, 85).unwrap();
    c.bench_function("decode_jpeg/512", |b| {
        b.iter(|| black_box(decode_jpeg(black_box(&encoded)).unwrap()))
    });
}

fn bench_encode_webp(c: &mut Criterion) {
    let buf = textured_buf_512();
    c.bench_function("encode_webp/512", |b| {
        b.iter(|| black_box(encode_webp(black_box(&buf)).unwrap()))
    });
}

fn bench_decode_webp(c: &mut Criterion) {
    let buf = textured_buf_512();
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
