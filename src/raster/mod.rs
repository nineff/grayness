use image::{
    DynamicImage, GenericImageView, ImageFormat, Pixel, RgbaImage, io::Reader as ImageReader,
};
use std::io::Cursor;
use wasm_minimal_protocol::{initiate_protocol, wasm_func};

initiate_protocol!();

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
