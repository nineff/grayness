use image::{
    DynamicImage, GenericImageView, ImageFormat, Pixel, RgbaImage, io::Reader as ImageReader,
};
use std::io::Cursor;
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
        .insert("id".into(), "TypstSVGFilter".into());
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
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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
            let target_alpha = pixel[3] as f32 / 255.0;
            let mask_alpha = if use_alpha {
                mask_pixel[3] as f32 / 255.0
            } else {
                mask_pixel.to_luma()[0] as f32 / 255.0
            };
            let new_alpha = (target_alpha * mask_alpha * 255.0).round() as u8;
            pixel[3] = new_alpha;
            output.put_pixel(x, y, pixel);
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    output
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
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
    let (mut img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let res = img.crop(start_x, start_y, width, height);

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
fn svg_crop(
    image_bytes: &[u8],
    start_x: &[u8],
    start_y: &[u8],
    width: &[u8],
    height: &[u8],
) -> Result<Vec<u8>, String> {
    let start_x = f32::from_le_bytes(
        start_x
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let start_y = f32::from_le_bytes(
        start_y
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let width = f32::from_le_bytes(
        width
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let height = f32::from_le_bytes(
        height
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;
    if svg_elem.attributes.contains_key("viewBox") {
        *svg_elem.attributes.get_mut("viewBox").unwrap() =
            format!("{start_x} {start_y} {width} {height}");
    } else {
        svg_elem.attributes.insert(
            "viewBox".to_string(),
            format!("{start_x} {start_y} {width} {height}"),
        );
    }

    let mut svg_output = Vec::new();
    svg_elem
        .write(&mut svg_output)
        .map_err(|e| format!("Could not write SVG bytes: {e:?}"))?;
    Ok(svg_output)
}

#[wasm_func]
pub fn blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let (img, mut format) = get_decoded_image_from_bytes(image_bytes)?;
    let sigma = f32::from_le_bytes(
        sigma
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );
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
fn svg_blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let sigma = f32::from_le_bytes(
        sigma
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    //create a gaussian blur filter
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstSVGFilter".into());
    let mut fe_gaussian_blur = Element::new("feGaussianBlur");
    fe_gaussian_blur
        .attributes
        .insert("stdDeviation".into(), format!("{sigma}"));

    filter_elem
        .children
        .push(XMLNode::Element(fe_gaussian_blur));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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
    res.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("Could not write image bytes to buffer: {e:?}"))?;

    Ok(bytes)
}

#[wasm_func]
fn svg_transparency(image_bytes: &[u8], alpha: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let alpha = f32::from_le_bytes(
        alpha
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    //create a component transfer filter for the alpha channel
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstSVGFilter".into());
    let mut fe_component_transfer = Element::new("feComponentTransfer");
    let mut fe_func_a = Element::new("feFuncA");
    fe_func_a.attributes.insert("type".into(), "linear".into());
    fe_func_a
        .attributes
        .insert("slope".into(), format!("{alpha}"));

    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_a));

    filter_elem
        .children
        .push(XMLNode::Element(fe_component_transfer));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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
fn svg_invert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    //create a component transfer filter for the RGB channels with invertion table
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstSVGFilter".into());
    filter_elem
        .attributes
        .insert("style".into(), "color-interpolation-filters:sRGB".into());
    let mut fe_component_transfer = Element::new("feComponentTransfer");
    let mut fe_func_r = Element::new("feFuncR");
    let mut fe_func_g = Element::new("feFuncG");
    let mut fe_func_b = Element::new("feFuncB");
    fe_func_r.attributes.insert("type".into(), "table".into());
    fe_func_r
        .attributes
        .insert("tableValues".into(), "1 0".into());
    fe_func_g.attributes.insert("type".into(), "table".into());
    fe_func_g
        .attributes
        .insert("tableValues".into(), "1 0".into());
    fe_func_b.attributes.insert("type".into(), "table".into());
    fe_func_b
        .attributes
        .insert("tableValues".into(), "1 0".into());

    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_r));
    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_g));
    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_b));

    filter_elem
        .children
        .push(XMLNode::Element(fe_component_transfer));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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
fn svg_brighten(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let amount = f32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    //create a component transfer filter for the RGB channels
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstSVGFilter".into());
    filter_elem
        .attributes
        .insert("style".into(), "color-interpolation-filters:sRGB".into());
    let mut fe_component_transfer = Element::new("feComponentTransfer");
    let mut fe_func_r = Element::new("feFuncR");
    let mut fe_func_g = Element::new("feFuncG");
    let mut fe_func_b = Element::new("feFuncB");
    fe_func_r.attributes.insert("type".into(), "linear".into());
    fe_func_r.attributes.insert("slope".into(), "1".into());
    fe_func_r
        .attributes
        .insert("intercept".into(), format!("{amount}"));
    fe_func_g.attributes.insert("type".into(), "linear".into());
    fe_func_g.attributes.insert("slope".into(), "1".into());
    fe_func_g
        .attributes
        .insert("intercept".into(), format!("{amount}"));
    fe_func_b.attributes.insert("type".into(), "linear".into());
    fe_func_b.attributes.insert("slope".into(), "1".into());
    fe_func_b
        .attributes
        .insert("intercept".into(), format!("{amount}"));

    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_r));
    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_g));
    fe_component_transfer
        .children
        .push(XMLNode::Element(fe_func_b));

    filter_elem
        .children
        .push(XMLNode::Element(fe_component_transfer));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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

#[wasm_func]
fn svg_huerotate(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let mut svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let amount = f32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    //create a gaussian blur filter
    let mut filter_elem = Element::new("filter");
    filter_elem
        .attributes
        .insert("id".into(), "TypstSVGFilter".into());
    let mut fe_color_matrix = Element::new("feColorMatrix ");
    fe_color_matrix
        .attributes
        .insert("type".into(), "hueRotate".into());
    fe_color_matrix
        .attributes
        .insert("values".into(), format!("{amount}deg"));

    filter_elem.children.push(XMLNode::Element(fe_color_matrix));

    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), "url(#TypstSVGFilter)".into());

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
