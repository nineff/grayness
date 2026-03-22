use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Pixel, RgbaImage};
use std::io::Cursor;
use wasm_minimal_protocol::wasm_func;

use crate::__BytesOrResultBytes;
use crate::__send_result_to_host;
use crate::__write_args_to_buffer;

fn write_image_buffer(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>, String> {
    let targetformat = match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP => format,
        _ => ImageFormat::Png,
    };

    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), targetformat)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
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

#[wasm_func]
pub fn grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.grayscale();
    write_image_buffer(&res, format)
}

#[wasm_func]
pub fn convert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    write_image_buffer(&img, format)
}

#[wasm_func]
pub fn decode(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, _) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.to_rgba8();
    Ok(res.to_vec())
}

#[wasm_func]
pub fn infos(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let (w, h) = img.dimensions();

    Ok([
        w.to_le_bytes().as_slice(),
        h.to_le_bytes().as_slice(),
        format!("{format:?}").as_bytes(),
    ]
    .concat())
}

#[wasm_func]
pub fn mask(
    target_image_bytes: &[u8],
    mask_image_bytges: &[u8],
    use_alpha: &[u8],
) -> Result<Vec<u8>, String> {
    let use_alpha = !use_alpha.is_empty() && use_alpha[0] != 0;
    let (targetimg, _) = get_decoded_image_from_bytes(target_image_bytes)?;
    let (mut mask, _) = get_decoded_image_from_bytes(mask_image_bytges)?;

    let (target_width, target_height) = targetimg.dimensions();
    if mask.dimensions() != targetimg.dimensions() {
        mask = mask.resize_exact(
            target_width,
            target_height,
            image::imageops::FilterType::Nearest,
        );
    }

    let mut output = RgbaImage::new(target_width, target_height);

    for y in 0..target_height {
        for x in 0..target_width {
            let mut pixel = targetimg.get_pixel(x, y);
            let mask_pixel = mask.get_pixel(x, y);
            let target_alpha = f32::from(pixel[3]) / 255.0;
            let mask_alpha = if use_alpha {
                f32::from(mask_pixel[3]) / 255.0
            } else {
                f32::from(mask_pixel.to_luma()[0]) / 255.0
            };

            //pixel values are always positive, no precision is lost since the floats were only intermediate anyway.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let new_alpha = (target_alpha * mask_alpha * 255.0).round() as u8;
            pixel[3] = new_alpha;
            output.put_pixel(x, y, pixel);
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    output
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png) //Always use PNG for its alpha channel
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
    let width = u32::from_le_bytes(
        width
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let height = u32::from_le_bytes(
        height
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let (mut img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.crop(start_x, start_y, width, height);

    write_image_buffer(&res, format)
}

#[wasm_func]
pub fn blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let sigma = f32::from_le_bytes(
        sigma
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let res = img.blur(sigma);
    write_image_buffer(&res, format)
}

#[wasm_func]
pub fn transparency(image_bytes: &[u8], alpha: &[u8]) -> Result<Vec<u8>, String> {
    let (img, _) = get_decoded_image_from_bytes(image_bytes)?;
    let alpha = u8::from_le_bytes(
        alpha
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let mut res = img.to_rgba8();

    for y in 0..res.height() {
        for x in 0..res.width() {
            let pixel = res.get_pixel_mut(x, y);
            pixel.apply_with_alpha(|ch| ch, |_| alpha);
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png) //Always use PNG for its alpha channel
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
pub fn invert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (mut img, format) = get_decoded_image_from_bytes(image_bytes)?;
    img.invert();
    write_image_buffer(&img, format)
}

#[wasm_func]
pub fn brighten(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let amount = i32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let res = img.brighten(amount);
    write_image_buffer(&res, format)
}

#[wasm_func]
pub fn huerotate(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let (img, format) = get_decoded_image_from_bytes(image_bytes)?;
    let amount = i32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to int: {e:?}"))?,
    );
    let res = img.huerotate(amount);
    write_image_buffer(&res, format)
}

#[wasm_func]
#[allow(clippy::too_many_arguments)]
pub fn matrix(
    image_bytes: &[u8],
    m00: &[u8],
    m01: &[u8],
    m02: &[u8],
    m03: &[u8],
    m04: &[u8],
    m10: &[u8],
    m11: &[u8],
    m12: &[u8],
    m13: &[u8],
    m14: &[u8],
    m20: &[u8],
    m21: &[u8],
    m22: &[u8],
    m23: &[u8],
    m24: &[u8],
    m30: &[u8],
    m31: &[u8],
    m32: &[u8],
    m33: &[u8],
    m34: &[u8],
) -> Result<Vec<u8>, String> {
    let (img, _format) = get_decoded_image_from_bytes(image_bytes)?;
    let mut res = img.to_rgba8();

    let m00 = f32::from_le_bytes(
        m00.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m01 = f32::from_le_bytes(
        m01.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m02 = f32::from_le_bytes(
        m02.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m03 = f32::from_le_bytes(
        m03.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m04 = f32::from_le_bytes(
        m04.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m10 = f32::from_le_bytes(
        m10.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m11 = f32::from_le_bytes(
        m11.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m12 = f32::from_le_bytes(
        m12.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m13 = f32::from_le_bytes(
        m13.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m14 = f32::from_le_bytes(
        m14.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m20 = f32::from_le_bytes(
        m20.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m21 = f32::from_le_bytes(
        m21.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m22 = f32::from_le_bytes(
        m22.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m23 = f32::from_le_bytes(
        m23.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m24 = f32::from_le_bytes(
        m24.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m30 = f32::from_le_bytes(
        m30.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m31 = f32::from_le_bytes(
        m31.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m32 = f32::from_le_bytes(
        m32.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m33 = f32::from_le_bytes(
        m33.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let m34 = f32::from_le_bytes(
        m34.try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    for y in 0..res.height() {
        for x in 0..res.width() {
            let pixel = res.get_pixel_mut(x, y);
            let r = f32::from(pixel[0]);
            let g = f32::from(pixel[1]);
            let b = f32::from(pixel[2]);
            let a = f32::from(pixel[3]);

            let nr = m00 * r + m01 * g + m02 * b + m03 * a + m04 * 255.0;
            let ng = m10 * r + m11 * g + m12 * b + m13 * a + m14 * 255.0;

            let nb = m20 * r + m21 * g + m22 * b + m23 * a + m24 * 255.0;
            let na = m30 * r + m31 * g + m32 * b + m33 * a + m34 * 255.0;

            pixel[0] = nr.clamp(0.0, 255.0) as u8;
            pixel[1] = ng.clamp(0.0, 255.0) as u8;
            pixel[2] = nb.clamp(0.0, 255.0) as u8;
            pixel[3] = na.clamp(0.0, 255.0) as u8;
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    res.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png) //Always use PNG for its alpha channel
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;
    Ok(bytes)
}
