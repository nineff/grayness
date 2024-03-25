use image::{DynamicImage, GenericImageView, ImageFormat, Pixel, io::Reader as ImageReader};
use std::{io::Cursor, str::from_utf8};
use wasm_minimal_protocol::*;

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
pub fn convert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;

    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { // Do nothing
        }
        _ => {
            format = ImageFormat::Png;
        }
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
pub fn rotate90(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.rotate90();

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
pub fn rotate180(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.rotate180();

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
pub fn rotate270(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.rotate270();

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
pub fn transparency(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let (mut img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let amount = bytes_to_int(amount)?;
    let (width, height) = img.dimensions();

    let res = img
        .as_mut_rgba8()
        .ok_or("Failed to convert image to RGBA8")?;

    for y in 0..height {
        for x in 0..width {
            let pixel_color = res.get_pixel_mut(x, y);
            pixel_color.apply_with_alpha(|rgb| rgb, |_| amount);
        }
    }

    match format {
        ImageFormat::Png => {}
        _ => {
            format = ImageFormat::Png;
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), format)
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
