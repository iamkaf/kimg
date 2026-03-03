use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::document::Document;
use kimg_core::layer::{
    FilterLayerData, GroupLayerData, ImageLayerData, Layer, LayerCommon, LayerKind, LayerTransform,
    PaintLayerData, ShapeLayerData, ShapeStroke, ShapeType, TextAlign, TextFontStyle,
    TextLayerData, TextWrap,
};
use kimg_core::pixel::Rgba;
use kimg_core::serialize::{deserialize, serialize};
#[cfg(feature = "cosmic-text-backend")]
use std::sync::Once;

fn solid_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(120, 80, 200, 255));
    buf
}

fn tinted_buf(size: u32, seed: u8, alpha: u8) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) as usize * 4;
            buf.data[i] = seed.wrapping_add(((x * 17 + y * 7) % 61) as u8);
            buf.data[i + 1] = seed.wrapping_add(((x * 11 + y * 13) % 47) as u8);
            buf.data[i + 2] = seed.wrapping_add(((x * 5 + y * 19) % 53) as u8);
            buf.data[i + 3] = alpha;
        }
    }
    buf
}

fn mask_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    let center = size as f64 / 2.0;
    let radius = center.max(1.0);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = ((dx * dx + dy * dy).sqrt() / radius).clamp(0.0, 1.0);
            let alpha = ((1.0 - dist) * 255.0).round() as u8;
            buf.set_pixel(x, y, Rgba::new(alpha, alpha, alpha, 255));
        }
    }
    buf
}

fn image_layer(id: u32, buf: ImageBuffer) -> Layer {
    Layer::new(
        LayerCommon::new(id, format!("layer-{id}")),
        LayerKind::Image(ImageLayerData::new(buf)),
    )
}

fn paint_layer(id: u32, buf: ImageBuffer) -> Layer {
    Layer::new(
        LayerCommon::new(id, format!("paint-{id}")),
        LayerKind::Paint(PaintLayerData::new(buf)),
    )
}

fn shape_layer(id: u32, size: u32) -> Layer {
    Layer::new(
        LayerCommon::new(id, format!("shape-{id}")),
        LayerKind::Shape(ShapeLayerData::new(
            ShapeType::RoundedRect,
            size,
            size,
            24,
            Some(Rgba::new(220, 80, 120, 255)),
            Some(ShapeStroke::new(Rgba::new(255, 255, 255, 255), 4)),
            Vec::new(),
        )),
    )
}

fn make_transform(
    anchor: kimg_core::blit::Anchor,
    flip_x: bool,
    flip_y: bool,
    rotation_deg: f64,
    scale_x: f64,
    scale_y: f64,
) -> LayerTransform {
    let mut transform = LayerTransform::new();
    transform.anchor = anchor;
    transform.flip_x = flip_x;
    transform.flip_y = flip_y;
    transform.rotation_deg = rotation_deg;
    transform.scale_x = scale_x;
    transform.scale_y = scale_y;
    transform
}

fn transformed_image_layer(id: u32, size: u32) -> Layer {
    let mut layer = image_layer(id, solid_buf(size));
    if let LayerKind::Image(image) = &mut layer.kind {
        image.transform = make_transform(
            kimg_core::blit::Anchor::Center,
            false,
            false,
            22.5,
            1.25,
            0.75,
        );
    }
    layer
}

fn transformed_paint_layer(id: u32, size: u32) -> Layer {
    let mut layer = paint_layer(id, solid_buf(size));
    if let LayerKind::Paint(paint) = &mut layer.kind {
        paint.transform =
            make_transform(kimg_core::blit::Anchor::Center, false, true, 15.0, 1.5, 0.8);
    }
    layer
}

fn transformed_shape_layer(id: u32, size: u32) -> Layer {
    let mut layer = shape_layer(id, size);
    if let LayerKind::Shape(shape) = &mut layer.kind {
        shape.transform = make_transform(
            kimg_core::blit::Anchor::Center,
            true,
            false,
            30.0,
            1.2,
            0.85,
        );
    }
    layer
}

fn text_layer(id: u32, name: &str, text: &str, color: Rgba, x: i32, y: i32) -> Layer {
    let mut common = LayerCommon::new(id, name);
    common.x = x;
    common.y = y;
    Layer::new(
        common,
        LayerKind::Text(TextLayerData::new(text, color, 16, 20, 0)),
    )
}

#[cfg(feature = "cosmic-text-backend")]
fn prepare_text_bench_fonts() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let fixtures = [
            include_bytes!("../../../tests/fixtures/inter-kimg.woff2").as_slice(),
            include_bytes!("../../../tests/fixtures/inter-cupcake-regular.woff2").as_slice(),
            include_bytes!("../../../tests/fixtures/inter-cupcake-bold.woff2").as_slice(),
            include_bytes!("../../../tests/fixtures/inter-cupcake-italic.woff2").as_slice(),
        ];

        for bytes in fixtures {
            let loaded = kimg_core::text::register_font_bytes(bytes.to_vec());
            assert!(loaded > 0, "expected bench font fixture to load");
        }
    });
}

#[cfg(not(feature = "cosmic-text-backend"))]
fn prepare_text_bench_fonts() {}

fn make_doc_single_image(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    let id = doc.next_id();
    doc.layers.push(image_layer(id, solid_buf(size)));
    doc
}

fn make_doc_10_layers(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    for _ in 0..10 {
        let id = doc.next_id();
        doc.layers.push(image_layer(id, solid_buf(size)));
    }
    doc
}

fn make_doc_10_normal_layers(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    for layer_index in 0..10u8 {
        let id = doc.next_id();
        doc.layers.push(image_layer(
            id,
            tinted_buf(size, layer_index.wrapping_mul(21), 224),
        ));
    }
    doc
}

fn make_doc_clipped_layer_stack(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    for layer_index in 0..10u8 {
        let id = doc.next_id();
        let mut layer = image_layer(id, tinted_buf(size, layer_index.wrapping_mul(19), 224));
        layer.common.clip_to_below = layer_index != 0;
        doc.layers.push(layer);
    }
    doc
}

fn make_doc_masked_layer_stack(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    let mask = mask_buf(size);
    for layer_index in 0..6u8 {
        let id = doc.next_id();
        let mut layer = image_layer(id, tinted_buf(size, layer_index.wrapping_mul(31), 255));
        layer.common.mask = Some(mask.clone());
        doc.layers.push(layer);
    }
    doc
}

fn make_doc_10_layers_with_filter(size: u32) -> Document {
    let mut doc = make_doc_10_layers(size);
    let id = doc.next_id();
    let mut filter_data = FilterLayerData::new();
    filter_data.config.hue_deg = 30.0;
    filter_data.config.saturation = 0.1;
    doc.layers.push(Layer::new(
        LayerCommon::new(id, "hsl"),
        LayerKind::Filter(filter_data),
    ));
    doc
}

fn make_doc_group_of_5(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    let group_id = doc.next_id();
    let mut group_data = GroupLayerData::new();
    for _ in 0..5 {
        let id = doc.next_id();
        group_data.children.push(image_layer(id, solid_buf(size)));
    }
    doc.layers.push(Layer::new(
        LayerCommon::new(group_id, "group"),
        LayerKind::Group(group_data),
    ));
    doc
}

fn make_doc_single_shape(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    let id = doc.next_id();
    doc.layers.push(shape_layer(id, size));
    doc
}

fn make_doc_10_shapes(size: u32) -> Document {
    let mut doc = Document::new(size, size);
    for _ in 0..10 {
        let id = doc.next_id();
        doc.layers.push(shape_layer(id, size));
    }
    doc
}

fn make_doc_10_shapes_with_filter(size: u32) -> Document {
    let mut doc = make_doc_10_shapes(size);
    let id = doc.next_id();
    let mut filter_data = FilterLayerData::new();
    filter_data.config.contrast = 0.2;
    filter_data.config.sharpen = 0.3;
    doc.layers.push(Layer::new(
        LayerCommon::new(id, "shape-filter"),
        LayerKind::Filter(filter_data),
    ));
    doc
}

fn make_doc_transformed_mix(size: u32) -> Document {
    let mut doc = Document::new(size, size);

    let image_id = doc.next_id();
    doc.layers.push(transformed_image_layer(image_id, size));

    let paint_id = doc.next_id();
    doc.layers.push(transformed_paint_layer(paint_id, size));

    let shape_id = doc.next_id();
    doc.layers.push(transformed_shape_layer(shape_id, size));

    for _ in 0..7 {
        let id = doc.next_id();
        doc.layers.push(transformed_image_layer(id, size));
    }

    doc
}

fn make_doc_registered_text() -> Document {
    prepare_text_bench_fonts();

    let mut doc = Document::new(320, 168);
    let paper_id = doc.next_id();
    doc.layers.push(Layer::new(
        LayerCommon::new(paper_id, "paper"),
        LayerKind::SolidColor(kimg_core::layer::SolidColorLayerData::new(Rgba::new(
            247, 241, 232, 255,
        ))),
    ));
    let backplate_id = doc.next_id();
    doc.layers.push(Layer::new(
        {
            let mut common = LayerCommon::new(backplate_id, "backplate");
            common.x = 14;
            common.y = 18;
            common
        },
        LayerKind::Shape(ShapeLayerData::new(
            ShapeType::RoundedRect,
            128,
            112,
            14,
            Some(Rgba::new(228, 113, 76, 28)),
            Some(ShapeStroke::new(Rgba::new(228, 113, 76, 96), 3)),
            Vec::new(),
        )),
    ));

    let mut headline = text_layer(
        doc.next_id(),
        "headline",
        "HELLO",
        Rgba::new(201, 73, 45, 255),
        28,
        32,
    );
    if let LayerKind::Text(text) = &mut headline.kind {
        text.font_family = "Inter".to_string();
        text.font_size = 24;
        text.line_height = 28;
        text.letter_spacing = 2;
    }
    doc.layers.push(headline);

    let mut badge = text_layer(
        doc.next_id(),
        "badge",
        "KIMG\nTEXT",
        Rgba::new(35, 79, 221, 255),
        236,
        98,
    );
    if let LayerKind::Text(text) = &mut badge.kind {
        text.font_family = "Inter".to_string();
        text.font_size = 24;
        text.line_height = 28;
        text.letter_spacing = 2;
        text.transform.anchor = kimg_core::blit::Anchor::Center;
        text.transform.rotation_deg = -12.0;
    }
    doc.layers.push(badge);

    doc
}

fn make_doc_styled_text_columns() -> Document {
    prepare_text_bench_fonts();

    let mut doc = Document::new(320, 176);
    let paper_id = doc.next_id();
    doc.layers.push(Layer::new(
        LayerCommon::new(paper_id, "paper"),
        LayerKind::SolidColor(kimg_core::layer::SolidColorLayerData::new(Rgba::new(
            247, 241, 232, 255,
        ))),
    ));

    let panel_specs = [
        (
            "left / 400",
            16,
            TextAlign::Left,
            TextFontStyle::Normal,
            400,
            Rgba::new(201, 73, 45, 255),
        ),
        (
            "center / 700",
            116,
            TextAlign::Center,
            TextFontStyle::Normal,
            700,
            Rgba::new(35, 79, 221, 255),
        ),
        (
            "right / italic",
            216,
            TextAlign::Right,
            TextFontStyle::Italic,
            400,
            Rgba::new(24, 119, 92, 255),
        ),
    ];
    let cupcake = "Cupcake ipsum\ndolor sit amet\nfrosting";

    for (name, x, align, font_style, font_weight, color) in panel_specs {
        let panel_id = doc.next_id();
        doc.layers.push(Layer::new(
            {
                let mut common = LayerCommon::new(panel_id, format!("{name}-panel"));
                common.x = x;
                common.y = 16;
                common
            },
            LayerKind::Shape(ShapeLayerData::new(
                ShapeType::RoundedRect,
                88,
                136,
                10,
                Some(Rgba::new(0, 0, 0, 0)),
                Some(ShapeStroke::new(Rgba::new(120, 112, 101, 90), 2)),
                Vec::new(),
            )),
        ));

        let mut text = text_layer(doc.next_id(), name, cupcake, color, x + 8, 28);
        if let LayerKind::Text(text_data) = &mut text.kind {
            text_data.font_family = "Inter".to_string();
            text_data.font_size = 16;
            text_data.line_height = 22;
            text_data.align = align;
            text_data.wrap = TextWrap::Word;
            text_data.box_width = Some(72);
            text_data.font_style = font_style;
            text_data.font_weight = font_weight;
        }
        doc.layers.push(text);
    }

    doc
}

fn bench_render_single_image(c: &mut Criterion) {
    let doc = make_doc_single_image(512);
    c.bench_function("render/single_image/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_layers(c: &mut Criterion) {
    let doc = make_doc_10_layers(512);
    c.bench_function("render/10_layers/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_normal_layers(c: &mut Criterion) {
    let doc = make_doc_10_normal_layers(512);
    c.bench_function("render/10_normal_layers/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_layers_with_filter(c: &mut Criterion) {
    let doc = make_doc_10_layers_with_filter(512);
    c.bench_function("render/10_layers_with_filter/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_single_shape(c: &mut Criterion) {
    let doc = make_doc_single_shape(512);
    c.bench_function("render/single_shape/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_shapes(c: &mut Criterion) {
    let doc = make_doc_10_shapes(512);
    c.bench_function("render/10_shapes/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_shapes_with_filter(c: &mut Criterion) {
    let doc = make_doc_10_shapes_with_filter(512);
    c.bench_function("render/10_shapes_with_filter/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_group_of_5(c: &mut Criterion) {
    let doc = make_doc_group_of_5(512);
    c.bench_function("render/group_of_5/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_transformed_image(c: &mut Criterion) {
    let mut doc = Document::new(512, 512);
    let id = doc.next_id();
    doc.layers.push(transformed_image_layer(id, 512));
    c.bench_function("render/transformed_image/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_transformed_paint(c: &mut Criterion) {
    let mut doc = Document::new(512, 512);
    let id = doc.next_id();
    doc.layers.push(transformed_paint_layer(id, 512));
    c.bench_function("render/transformed_paint/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_transformed_shape(c: &mut Criterion) {
    let mut doc = Document::new(512, 512);
    let id = doc.next_id();
    doc.layers.push(transformed_shape_layer(id, 512));
    c.bench_function("render/transformed_shape/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_10_layers_with_transforms(c: &mut Criterion) {
    let doc = make_doc_transformed_mix(512);
    c.bench_function("render/10_layers_with_transforms/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_clipped_layer_stack(c: &mut Criterion) {
    let doc = make_doc_clipped_layer_stack(512);
    c.bench_function("render/clipped_layer_stack/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_masked_layer_stack(c: &mut Criterion) {
    let doc = make_doc_masked_layer_stack(512);
    c.bench_function("render/masked_layer_stack/512", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_repeated_transformed_layer(c: &mut Criterion) {
    let mut doc = Document::new(512, 512);
    let id = doc.next_id();
    doc.layers.push(transformed_image_layer(id, 512));
    c.bench_function("render/repeated_transformed_layer/512", |b| {
        b.iter(|| {
            black_box(doc.render());
            black_box(doc.render())
        })
    });
}

fn bench_render_registered_text_cold(c: &mut Criterion) {
    c.bench_function("render/text_registered_cold/320x168", |b| {
        b.iter_batched(
            make_doc_registered_text,
            |doc| black_box(doc.render()),
            BatchSize::SmallInput,
        )
    });
}

fn bench_render_registered_text_cached(c: &mut Criterion) {
    let doc = make_doc_registered_text();
    c.bench_function("render/text_registered_cached/320x168", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_text_styles_cold(c: &mut Criterion) {
    c.bench_function("render/text_styles_cold/320x176", |b| {
        b.iter_batched(
            make_doc_styled_text_columns,
            |doc| black_box(doc.render()),
            BatchSize::SmallInput,
        )
    });
}

fn bench_render_text_styles_cached(c: &mut Criterion) {
    let doc = make_doc_styled_text_columns();
    c.bench_function("render/text_styles_cached/320x176", |b| {
        b.iter(|| black_box(doc.render()))
    });
}

fn bench_render_repeated_text(c: &mut Criterion) {
    let doc = make_doc_styled_text_columns();
    c.bench_function("render/repeated_text_styles/320x176", |b| {
        b.iter(|| {
            black_box(doc.render());
            black_box(doc.render())
        })
    });
}

fn bench_serialize_deserialize(c: &mut Criterion) {
    let doc = make_doc_10_layers(512);
    c.bench_function("serialize_deserialize/10_layers", |b| {
        b.iter(|| {
            let bytes = serialize(black_box(&doc)).unwrap();
            let restored = deserialize(black_box(&bytes)).unwrap();
            black_box(restored)
        })
    });
}

criterion_group!(
    benches,
    bench_render_single_image,
    bench_render_10_layers,
    bench_render_10_normal_layers,
    bench_render_10_layers_with_filter,
    bench_render_single_shape,
    bench_render_10_shapes,
    bench_render_10_shapes_with_filter,
    bench_render_group_of_5,
    bench_render_transformed_image,
    bench_render_transformed_paint,
    bench_render_transformed_shape,
    bench_render_10_layers_with_transforms,
    bench_render_clipped_layer_stack,
    bench_render_masked_layer_stack,
    bench_render_repeated_transformed_layer,
    bench_render_registered_text_cold,
    bench_render_registered_text_cached,
    bench_render_text_styles_cold,
    bench_render_text_styles_cached,
    bench_render_repeated_text,
    bench_serialize_deserialize
);
criterion_main!(benches);
