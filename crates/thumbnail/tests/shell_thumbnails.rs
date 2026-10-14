//! Thumbnails from the system, as the file manager shows them.

use std::path::Path;

use sbm_thumbnail::thumbnail;

/// Writes a `side`×`side` PNG of one colour.
fn write_png(path: &Path, side: u32, rgb: [u8; 3]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, side, side);
    encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    let pixels: Vec<u8> = (0..side * side).flat_map(|_| rgb).collect();
    writer.write_image_data(&pixels).unwrap();
}

/// Width, height and RGBA pixels of a PNG.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    (info.width, info.height, pixels)
}

#[test]
#[cfg(windows)]
fn picture_gets_a_thumbnail_of_what_it_shows() {
    let folder = tempfile::tempdir().unwrap();
    let picture = folder.path().join("red.png");
    write_png(&picture, 256, [220, 30, 30]);

    let png = thumbnail(&picture, 64, false).expect("a thumbnail");

    let (width, height, pixels) = decode(&png);
    assert!(width <= 64 && height <= 64 && width > 0);
    let middle = (((height / 2) * width + width / 2) * 4) as usize;
    let [r, g, b, a] = [
        pixels[middle],
        pixels[middle + 1],
        pixels[middle + 2],
        pixels[middle + 3],
    ];
    assert!(r > 180 && g < 80 && b < 80 && a == 255, "{r} {g} {b} {a}");
}
