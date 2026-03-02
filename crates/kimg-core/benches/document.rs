use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::document::Document;
use kimg_core::layer::{
    FilterLayerData, GroupLayerData, ImageLayerData, Layer, LayerCommon, LayerKind, LayerTransform,
    PaintLayerData, ShapeLayerData, ShapeStroke, ShapeType,
};
use kimg_core::pixel::Rgba;
use kimg_core::serialize::{deserialize, serialize};

fn solid_buf(size: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(size, size);
    buf.fill(Rgba::new(120, 80, 200, 255));
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
    bench_render_10_layers_with_filter,
    bench_render_single_shape,
    bench_render_10_shapes,
    bench_render_10_shapes_with_filter,
    bench_render_group_of_5,
    bench_render_transformed_image,
    bench_render_transformed_paint,
    bench_render_transformed_shape,
    bench_render_10_layers_with_transforms,
    bench_serialize_deserialize
);
criterion_main!(benches);
