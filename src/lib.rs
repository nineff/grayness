use image::{io::Reader as ImageReader, DynamicImage, ImageFormat, Pixel};
use std::{io::Cursor, str::from_utf8, u8};
use wasm_minimal_protocol::*;
use xmltree::{Element, XMLNode};

initiate_protocol!();

#[wasm_func]
pub fn grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.grayscale();

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
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

    //add filter and replace existing children with new group
    svg_elem.children = vec![
        XMLNode::Element(filter_elem),
        XMLNode::Element(group_element),
    ];

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
    mode: &[u8],
) -> Result<Vec<u8>, String> {
    let start_x = u32::from_le_bytes(
        start_x
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let start_y = u32::from_le_bytes(
        start_y
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let mut width: f32 = utf8bytes_to_number(width)?;
    let mut height: f32 = utf8bytes_to_number(height)?;
    let (mut img, mut format) = get_decoded_image_from_bytes(image_bytes)?;

    if *mode.first().ok_or("Mode is not set".to_string())? == 1 {
        let original_height = img.height();
        let original_width = img.width();
        width *= original_width as f32;
        height *= original_height as f32;
    }
    let res = img.crop(start_x, start_y, width as _, height as _);

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let sigma = utf8bytes_to_number(sigma)?;
    let res = img.blur(sigma);

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn transparency(image_bytes: &[u8], alpha: &[u8]) -> Result<Vec<u8>, String> {
    let (img, _) = get_decoded_image_from_bytes(image_bytes)?;
    let alpha = utf8bytes_to_number(alpha)?;
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

#[wasm_func]
pub fn invert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (mut img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    img.invert();

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
pub fn brighten(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let amount = i32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let res = img.brighten(amount);

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn huerotate(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let amount = i32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let res = img.huerotate(amount);

    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        format = ImageFormat::Png;
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

fn utf8bytes_to_number<T>(bytes: &[u8]) -> Result<T, String>
where
    T: std::str::FromStr + std::fmt::Debug,
    T::Err: std::fmt::Debug,
{
    match from_utf8(bytes) {
        Ok(input) => input
            .parse()
            .map_err(|e| format!("String '{input}' could not be parsed as number: {e:?}")),
        Err(e) => Err(format!("Invalid UTF8: {e:?}")),
    }
}

fn get_decoded_image_from_bytes(bytes: &[u8]) -> Result<(DynamicImage, ImageFormat), String> {
    let img_r = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("Guessing the image format failed: {e:?}"))?;
    let format = img_r.format().ok_or("Unknown image format".to_string())?;
    let decoded = img_r
        .decode()
        .map_err(|e| format!("Could not decode image data: {e:?}"))?;
    Ok((decoded, format))
}
