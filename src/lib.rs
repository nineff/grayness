use image::{io::Reader as ImageReader, DynamicImage, ImageFormat, Pixel};
use std::{io::Cursor, str::from_utf8};
use wasm_minimal_protocol::*;
use xmltree::{Element, XMLNode};

initiate_protocol!();

#[wasm_func]
pub fn grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.grayscale();

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
fn svg_grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    //create a filter element with a colormatrix
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstGrayscaleFilter".into());
    let mut colormatrix_elem = Element::new("feColorMatrix");
    colormatrix_elem
        .attributes
        .insert("type".into(), "matrix".into());
    colormatrix_elem.attributes.insert(
        "values".into(),
        "0.3333 0.3333 0.3333 0 0 0.3333 0.3333 0.3333 0 0 0.3333 0.3333 0.3333 0 0 0 0 0 1 0"
            .into(), //see https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feColorMatrix
    );

    filter_elem
        .children
        .push(XMLNode::Element(colormatrix_elem));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstGrayscaleFilter)".into());

    for child in svg_elem.children {
        if let XMLNode::Element(elem) = child {
            group_element.children.push(XMLNode::Element(elem));
        }
    }

    svg_elem.children = vec![XMLNode::Element(group_element)];

    svg_elem.children.insert(0, XMLNode::Element(filter_elem));

    let mut svg_output = Vec::new();

    svg_elem
        .write(&mut svg_output)
        .map_err(|e| format!("Could not write SVG bytes: {e:?}"))?;
    Ok(svg_output)
}

#[wasm_func]
pub fn convert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
    }

    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn crop(
    image_bytes: &[u8],
    start_x: &[u8],
    start_y: &[u8],
    width: &[u8],
    height: &[u8],
) -> Result<Vec<u8>, String> {
    let start_x = bytes_to_int(start_x)?;
    let start_y = bytes_to_int(start_y)?;
    let width = bytes_to_int(width)?;
    let height = bytes_to_int(height)?;
    let (mut img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.crop(start_x, start_y, width, height);

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let sigma = bytes_to_int(sigma)?;
    let res = img.blur(sigma);

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn flipv(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.flipv();

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn fliph(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.fliph();

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn transparency(image_bytes: &[u8], alpha: &[u8]) -> Result<Vec<u8>, String> {
    let (img, _) = get_decoded_image_from_bytes(image_bytes)?;
    let alpha = bytes_to_int(alpha)?;
    let mut res = img.to_rgba8();

    for y in 0..res.height() {
        for x in 0..res.width() {
            let pixel = res.get_pixel_mut(x, y);
            pixel.apply_with_alpha(|ch| ch, |_| alpha);
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

fn bytes_to_int<T>(bytes: &[u8]) -> Result<T, String>
where
    T: std::str::FromStr + std::fmt::Debug,
    T::Err: std::fmt::Debug,
{
    match from_utf8(bytes) {
        Ok(input) => input
            .parse()
            .map_err(|e| format!("String could not be parsed as int: {e:?}")),
        Err(e) => Err(format!("Invalid UTF8: {e:?}")),
    }
}

fn get_decoded_image_from_bytes(bytes: &[u8]) -> Result<(DynamicImage, ImageFormat), String> {
    let img_r = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("Could not guess image format: {e:?}"))?;
    let format = img_r.format().ok_or("No Format".to_string())?;
    let decoded = img_r
        .decode()
        .map_err(|e| format!("Could not decode image data: {e:?}"))?;
    Ok((decoded, format))
}
