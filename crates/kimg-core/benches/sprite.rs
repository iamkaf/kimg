use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::pixel::Rgba;
use kimg_core::sprite::{extract_palette, pack_sprites, pixel_scale, quantize};

fn solid_buf(w: u32, h: u32, color: Rgba) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(w, h);
    buf.fill(color);
    buf
}

/// Build a 512×512 image with varied colors for realistic palette/quantize benchmarks.
fn varied_buf_512() -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(512, 512);
    for y in 0..512u32 {
        for x in 0..512u32 {
            let r = ((x * 255) / 511) as u8;
            let g = ((y * 255) / 511) as u8;
            let b = (((x + y) * 255) / 1022) as u8;
            let i = (y * 512 + x) as usize * 4;
            buf.data[i] = r;
            buf.data[i + 1] = g;
            buf.data[i + 2] = b;
            buf.data[i + 3] = 255;
        }
    }
    buf
}

fn bench_pack_sprites_16(c: &mut Criterion) {
    let sprites: Vec<ImageBuffer> = (0..16)
        .map(|i| solid_buf(32, 32, Rgba::new(i * 16, 100, 200, 255)))
        .collect();
    let refs: Vec<&ImageBuffer> = sprites.iter().collect();

    c.bench_function("pack_sprites/16sprites/32px", |b| {
        b.iter(|| black_box(pack_sprites(black_box(&refs), 1, 4096, false)))
    });
}

fn bench_pack_sprites_64(c: &mut Criterion) {
    let sprites: Vec<ImageBuffer> = (0..64)
        .map(|i| solid_buf(32, 32, Rgba::new((i * 4) as u8, 100, 200, 255)))
        .collect();
    let refs: Vec<&ImageBuffer> = sprites.iter().collect();

    c.bench_function("pack_sprites/64sprites/32px", |b| {
        b.iter(|| black_box(pack_sprites(black_box(&refs), 1, 4096, false)))
    });
}

fn bench_extract_palette(c: &mut Criterion) {
    let buf = varied_buf_512();
    c.bench_function("extract_palette/512/16colors", |b| {
        b.iter(|| black_box(extract_palette(black_box(&buf), 16)))
    });
}

fn bench_quantize(c: &mut Criterion) {
    let buf = varied_buf_512();
    let palette = extract_palette(&buf, 16);
    c.bench_function("quantize/512/16colors", |b| {
        b.iter(|| black_box(quantize(black_box(&buf), black_box(&palette))))
    });
}

fn bench_pixel_scale(c: &mut Criterion) {
    let buf = solid_buf(512, 512, Rgba::new(100, 200, 50, 255));
    c.bench_function("pixel_scale/512/2x", |b| {
        b.iter(|| black_box(pixel_scale(black_box(&buf), 2)))
    });
}

criterion_group!(
    benches,
    bench_pack_sprites_16,
    bench_pack_sprites_64,
    bench_extract_palette,
    bench_quantize,
    bench_pixel_scale
);
criterion_main!(benches);
