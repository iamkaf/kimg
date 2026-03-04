use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kimg_core::brush::{
    paint_stroke, BrushPreset, BrushSmoothingMode, BrushStrokeSession, BrushTip, BrushTool,
    StrokePoint,
};
use kimg_core::buffer::ImageBuffer;
use kimg_core::pixel::Rgba;

fn pressure_wave(length: usize, width: f32, height: f32) -> Vec<StrokePoint> {
    (0..length)
        .map(|index| {
            let t = index as f32 / (length.saturating_sub(1).max(1)) as f32;
            StrokePoint::new(
                t * width,
                height * 0.5 + (t * std::f32::consts::TAU * 2.0).sin() * height * 0.2,
                0.2 + t * 0.8,
            )
        })
        .collect()
}

fn short_stroke(origin_x: f32, origin_y: f32) -> Vec<StrokePoint> {
    vec![
        StrokePoint::new(origin_x, origin_y, 0.4),
        StrokePoint::new(origin_x + 12.0, origin_y + 4.0, 0.7),
        StrokePoint::new(origin_x + 24.0, origin_y + 2.0, 1.0),
    ]
}

fn brush_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("brush");

    group.bench_function("round_hard_small/256", |b| {
        let points = pressure_wave(24, 220.0, 256.0);
        let preset = BrushPreset {
            color: Rgba::new(201, 73, 45, 255),
            size: 8.0,
            hardness: 1.0,
            spacing: 0.35,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || ImageBuffer::new_transparent(256, 256),
            |mut buffer| {
                paint_stroke(
                    black_box(&mut buffer),
                    black_box(&preset),
                    black_box(&points),
                );
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("round_soft_large/512", |b| {
        let points = pressure_wave(48, 470.0, 512.0);
        let preset = BrushPreset {
            color: Rgba::new(35, 79, 221, 255),
            flow: 0.7,
            hardness: 0.2,
            pressure_opacity: 0.4,
            pressure_size: 1.0,
            size: 22.0,
            smoothing: 0.2,
            spacing: 0.25,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || ImageBuffer::new_transparent(512, 512),
            |mut buffer| {
                paint_stroke(
                    black_box(&mut buffer),
                    black_box(&preset),
                    black_box(&points),
                );
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("erase_soft/512", |b| {
        let points = pressure_wave(40, 460.0, 512.0);
        let preset = BrushPreset {
            tool: BrushTool::Erase,
            hardness: 0.35,
            size: 20.0,
            spacing: 0.25,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || {
                let mut buffer = ImageBuffer::new_transparent(512, 512);
                buffer.fill(Rgba::new(90, 120, 200, 255));
                buffer
            },
            |mut buffer| {
                paint_stroke(
                    black_box(&mut buffer),
                    black_box(&preset),
                    black_box(&points),
                );
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("long_pressure_stroke/1024", |b| {
        let points = pressure_wave(160, 980.0, 1024.0);
        let preset = BrushPreset {
            color: Rgba::new(0, 0, 0, 255),
            flow: 0.8,
            hardness: 0.55,
            pressure_opacity: 0.45,
            pressure_size: 1.0,
            size: 18.0,
            smoothing: 0.15,
            spacing: 0.3,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || ImageBuffer::new_transparent(1024, 1024),
            |mut buffer| {
                paint_stroke(
                    black_box(&mut buffer),
                    black_box(&preset),
                    black_box(&points),
                );
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("streamed_long_pressure_stroke/1024", |b| {
        let points = pressure_wave(160, 980.0, 1024.0);
        let chunks = [
            &points[..48],
            &points[48..96],
            &points[96..128],
            &points[128..],
        ];
        let preset = BrushPreset {
            color: Rgba::new(0, 0, 0, 255),
            flow: 0.8,
            hardness: 0.55,
            pressure_opacity: 0.45,
            pressure_size: 1.0,
            size: 18.0,
            smoothing: 0.15,
            spacing: 0.3,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || ImageBuffer::new_transparent(1024, 1024),
            |mut buffer| {
                let mut session = BrushStrokeSession::new(preset);
                for chunk in &chunks {
                    session.apply_points(black_box(&mut buffer), black_box(chunk));
                }
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("grain_tilt_modeler/512", |b| {
        let points = pressure_wave(48, 470.0, 512.0)
            .into_iter()
            .enumerate()
            .map(|(index, point)| {
                StrokePoint::with_tilt_time(
                    point.x,
                    point.y,
                    point.pressure,
                    -0.3 + (index as f32 / 47.0) * 1.3,
                    0.8 - (index as f32 / 47.0) * 0.6,
                    index as f32 * 16.0,
                )
            })
            .collect::<Vec<_>>();
        let preset = BrushPreset {
            color: Rgba::new(183, 132, 52, 255),
            flow: 0.75,
            hardness: 0.4,
            pressure_opacity: 0.35,
            pressure_size: 1.0,
            size: 18.0,
            smoothing: 0.4,
            smoothing_mode: BrushSmoothingMode::Modeler,
            spacing: 0.28,
            tip: BrushTip::Grain,
            ..BrushPreset::default()
        };

        b.iter_batched(
            || ImageBuffer::new_transparent(512, 512),
            |mut buffer| {
                paint_stroke(
                    black_box(&mut buffer),
                    black_box(&preset),
                    black_box(&points),
                );
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("repeated_short_strokes/512", |b| {
        let preset = BrushPreset {
            color: Rgba::new(242, 190, 62, 255),
            hardness: 0.8,
            size: 10.0,
            spacing: 0.4,
            ..BrushPreset::default()
        };
        let strokes = [
            short_stroke(24.0, 40.0),
            short_stroke(80.0, 88.0),
            short_stroke(144.0, 120.0),
            short_stroke(220.0, 180.0),
            short_stroke(300.0, 260.0),
            short_stroke(360.0, 320.0),
        ];

        b.iter_batched(
            || ImageBuffer::new_transparent(512, 512),
            |mut buffer| {
                for stroke in &strokes {
                    paint_stroke(
                        black_box(&mut buffer),
                        black_box(&preset),
                        black_box(stroke),
                    );
                }
                black_box(buffer);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, brush_benches);
criterion_main!(benches);
