use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kimg_core::buffer::ImageBuffer;
use kimg_core::document::Document;
use kimg_core::layer::{
    FilterLayerData, GroupLayerData, ImageLayerData, Layer, LayerCommon, LayerKind,
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

fn bench_render_group_of_5(c: &mut Criterion) {
    let doc = make_doc_group_of_5(512);
    c.bench_function("render/group_of_5/512", |b| {
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
    bench_render_group_of_5,
    bench_serialize_deserialize
);
criterion_main!(benches);
