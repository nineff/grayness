use wasm_minimal_protocol::wasm_func;
use xmltree::{Element, XMLNode};

use crate::__BytesOrResultBytes;
use crate::__send_result_to_host;
use crate::__write_args_to_buffer;

static TYPST_FILTER_ID_PREFIX: &str = "Typst_Filter_ID_";

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

fn add_svg_filter(
    mut svg_elem: Element,
    id: &str,
    filter_elem: Element,
) -> Result<Vec<u8>, String> {
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

    let mut svg_output = Vec::new();

    svg_elem
        .write(&mut svg_output)
        .map_err(|e| format!("Could not write SVG bytes: {e:?}"))?;
    Ok(svg_output)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
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

    add_svg_filter(svg_elem, &id, filter_elem)
}
