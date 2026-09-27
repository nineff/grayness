//   Copyright 2026 Nikolai Neff-Sarnow
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//	   http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.use wasm_minimal_protocol::{initiate_protocol, wasm_func};

use ciborium::cbor;
use ciborium::into_writer;
use image::ImageFormat;
use wasm_minimal_protocol::wasm_func;
use xmltree::{Element, XMLNode};

use crate::__ToResult;
use crate::__send_result_to_host;
use crate::__write_args_to_buffer;
use crate::raster::get_decoded_image_from_bytes;
use crate::raster::write_image_buffer;

static TYPST_FILTER_ID_PREFIX: &str = "Typst_Filter_ID_";
static TYPST_MASK_ID_PREFIX: &str = "Typst_Mask_ID_";

fn get_next_filter_index(root: &Element) -> usize {
    let mut max_n = 0;

    //look through every g element with a filter attribute matching the specified format and extract the maximum ID
    for child in &root.children {
        let XMLNode::Element(elem) = child else {
            continue;
        };

        if elem.name != "g" {
            continue;
        }
        let prefix = format!("url(#{TYPST_FILTER_ID_PREFIX}");
        let suffix = ")";
        if let Some(id) = elem.attributes.get("filter")
            && let Some(rest) = id.strip_prefix(&prefix)
            && let Some(num) = rest.strip_suffix(suffix)
            && let Ok(n) = num.parse::<usize>()
        {
            max_n = max_n.max(n);
        }
    }

    max_n + 1
}

fn get_next_mask_index(root: &Element) -> usize {
    let mut max_n = 0;

    //look through every mask element with an id matching the specified format and extract the maximum ID
    for child in &root.children {
        let XMLNode::Element(elem) = child else {
            continue;
        };

        if elem.name != "mask" {
            continue;
        }
        if let Some(id) = elem.attributes.get("id")
            && let Some(num) = id.strip_prefix(TYPST_MASK_ID_PREFIX)
            && let Ok(n) = num.parse::<usize>()
        {
            max_n = max_n.max(n);
        }
    }

    max_n + 1
}

fn write_to_vec(svg_elem: &Element) -> Result<Vec<u8>, String> {
    let mut svg_output = Vec::new();

    svg_elem
        .write(&mut svg_output)
        .map_err(|e| format!("Could not write SVG bytes: {e:?}"))?;
    Ok(svg_output)
}

fn add_svg_filter(
    mut svg_elem: Element,
    id: &str,
    filter_elem: Element,
) -> Result<Element, String> {
    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("filter".into(), format!("url(#{id})"));

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
    Ok(svg_elem)
}

fn add_svg_mask(mut svg_elem: Element, id: &str, mask_element: Element) -> Result<Element, String> {
    //wrap all existing elements in a new group with the filter applied
    let mut group_element = Element::new("g");
    group_element
        .attributes
        .insert("mask".into(), format!("url(#{id})"));

    for child in svg_elem.children {
        if let XMLNode::Element(elem) = child {
            group_element.children.push(XMLNode::Element(elem));
        }
    }

    //add mask and replace existing children with new group
    svg_elem.children = vec![
        XMLNode::Element(mask_element),
        XMLNode::Element(group_element),
    ];
    Ok(svg_elem)
}

#[wasm_func]
fn svg_grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;
    let num = get_next_filter_index(&svg_elem);

    //create a filter element with a colormatrix
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
    let mut colormatrix_elem = Element::new("feColorMatrix");
    colormatrix_elem
        .attributes
        .insert("type".into(), "saturate".into());
    colormatrix_elem.attributes.insert(
        "values".into(),
        "0.0".into(), //see https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feColorMatrix
    );
    filter_elem
        .children
        .push(XMLNode::Element(colormatrix_elem));

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
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

    write_to_vec(&svg_elem)
}

#[wasm_func]
fn svg_blur(image_bytes: &[u8], sigma: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let sigma = f32::from_le_bytes(
        sigma
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    let num = get_next_filter_index(&svg_elem);
    //create a gaussian blur filter
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
    let mut fe_gaussian_blur = Element::new("feGaussianBlur");
    fe_gaussian_blur
        .attributes
        .insert("stdDeviation".into(), format!("{sigma}"));

    filter_elem
        .children
        .push(XMLNode::Element(fe_gaussian_blur));

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
fn svg_transparency(image_bytes: &[u8], alpha: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let alpha = f32::from_le_bytes(
        alpha
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    let num = get_next_filter_index(&svg_elem);
    //create a component transfer filter for the alpha channel
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
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

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
fn svg_invert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let num = get_next_filter_index(&svg_elem);
    //create a component transfer filter for the RGB channels with inversion table
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
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

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
fn svg_brighten(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let amount = f32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    let num = get_next_filter_index(&svg_elem);
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    //create a component transfer filter for the RGB channels
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
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

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
fn svg_huerotate(image_bytes: &[u8], amount: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let amount = f32::from_le_bytes(
        amount
            .try_into()
            .map_err(|e| format!("could not convert bytes to float: {e:?}"))?,
    );

    let num = get_next_filter_index(&svg_elem);
    //create a Hue-rotating filter
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
    let mut fe_color_matrix = Element::new("feColorMatrix");
    fe_color_matrix
        .attributes
        .insert("type".into(), "hueRotate".into());
    fe_color_matrix
        .attributes
        .insert("values".into(), format!("{amount}"));

    filter_elem.children.push(XMLNode::Element(fe_color_matrix));

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
#[allow(clippy::too_many_arguments)]
fn svg_matrix(
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

    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    let num = get_next_filter_index(&svg_elem);
    //create a Hue-rotating filter
    let id = format!("{TYPST_FILTER_ID_PREFIX}{num}");
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.clone());
    let mut fe_color_matrix = Element::new("feColorMatrix");
    fe_color_matrix
        .attributes
        .insert("type".into(), "matrix".into());
    fe_color_matrix
        .attributes
        .insert("values".into(), format!("{m00} {m01} {m02} {m03} {m04} {m10} {m11} {m12} {m13} {m14} {m20} {m21} {m22} {m23} {m24} {m30} {m31} {m32} {m33} {m34}"));

    filter_elem.children.push(XMLNode::Element(fe_color_matrix));

    write_to_vec(&add_svg_filter(svg_elem, &id, filter_elem)?)
}

#[wasm_func]
fn svg_mask(image_bytes: &[u8], mask_bytes: &[u8]) -> Result<Vec<u8>, String> {
    use base64::prelude::*;

    let mask = BASE64_STANDARD.encode(write_image_buffer(
        &get_decoded_image_from_bytes(mask_bytes)?.0,
        ImageFormat::Png,
    )?);

    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;
    let num = get_next_mask_index(&svg_elem);

    //create a mask element with the image as child
    let id = format!("{TYPST_MASK_ID_PREFIX}{num}");
    let mut mask_elem = Element::new("mask");
    mask_elem.attributes.extend([
        ("id".into(), id.clone()),
        ("x".into(), "0".into()),
        ("y".into(), "0".into()),
        ("width".into(), "100%".into()),
        ("height".into(), "100%".into()),
    ]);
    let mut image_elem = Element::new("image");
    image_elem
        .attributes
        .insert("href".into(), format!("data:image/png;base64,{mask}")); //embedding the image instead of linking to it to maintain the api of passing raw bytes
    image_elem.attributes.extend([
        ("width".into(), "100%".into()),
        ("height".into(), "100%".into()),
    ]);
    mask_elem.children.push(XMLNode::Element(image_elem));

    write_to_vec(&add_svg_mask(svg_elem, &id, mask_elem)?)
}
#[wasm_func]
fn svg_infos(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;
    let w = svg_elem.attributes.get("width");
    let h = svg_elem.attributes.get("height");
    let view_box = svg_elem.attributes.get("viewBox");

    let cbor = cbor!({
        "w" => w,
        "h" => h,
        "viewBox" => view_box,
    })
    .map_err(|e| format!("Could not serialize svgInfos to CBOR: {e}"))?;
    let mut out = Vec::new();
    into_writer(&cbor, &mut out).map_err(|e| format!("could not write cbor: {e}"))?;
    Ok(out)
}

//#[cfg(all(test, not(target_arch = "wasm32")))]
#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_getting_next_mask_index() {
        let minimal_svg =
            Element::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#.as_bytes())
                .expect("SVG can be parsed");
        //first free index is 1
        assert_eq!(get_next_mask_index(&minimal_svg), 1);
        //add minimal mask with random ID (choosen by fair dice roll)
        let id = format!("{TYPST_MASK_ID_PREFIX}4");
        let mut mask_elem = Element::new("mask");
        mask_elem.attributes.insert("id".into(), id.clone());
        let res = add_svg_mask(minimal_svg, &id, mask_elem).expect("Adding Mask works");
        assert_eq!(get_next_mask_index(&res), 5);
    }
    #[test]
    fn test_gettig_next_filder_index() {
        let minimal_svg =
            Element::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#.as_bytes())
                .expect("SVG can be parsed");
        //first free index is 1
        assert_eq!(get_next_filter_index(&minimal_svg), 1);

        let filter_elem = Element::new("filter");
        let f = add_svg_filter(
            minimal_svg,
            &format!("{TYPST_FILTER_ID_PREFIX}7"),
            filter_elem,
        )
        .expect("Adding Filter Works");
        assert_eq!(get_next_filter_index(&f), 8);
    }
}
