use uuid::Uuid;
use wasm_minimal_protocol::{initiate_protocol, wasm_func};
use xmltree::{Element, XMLNode};

fn add_svg_filter(
    mut svg_elem: Element,
    id: Uuid,
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

initiate_protocol!();

#[wasm_func]
fn svg_grayscale(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    //create a filter element with a colormatrix
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
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

    add_svg_filter(svg_elem, id, filter_elem)
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

    //create a gaussian blur filter
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
    let mut fe_gaussian_blur = Element::new("feGaussianBlur");
    fe_gaussian_blur
        .attributes
        .insert("stdDeviation".into(), format!("{sigma}"));

    filter_elem
        .children
        .push(XMLNode::Element(fe_gaussian_blur));

    add_svg_filter(svg_elem, id, filter_elem)
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

    //create a component transfer filter for the alpha channel
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
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

    add_svg_filter(svg_elem, id, filter_elem)
}

#[wasm_func]
fn svg_invert(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let svg_elem =
        Element::parse(image_bytes).map_err(|e| format!("Could not parse SVG data: {e:?}"))?;

    //create a component transfer filter for the RGB channels with invertion table
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
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

    add_svg_filter(svg_elem, id, filter_elem)
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

    //create a component transfer filter for the RGB channels
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
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

    add_svg_filter(svg_elem, id, filter_elem)
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

    //create a Hue-rotating filter
    let id = Uuid::new_v4();
    let mut filter_elem = Element::new("filter");
    filter_elem.attributes.insert("id".into(), id.into());
    let mut fe_color_matrix = Element::new("feColorMatrix");
    fe_color_matrix
        .attributes
        .insert("type".into(), "hueRotate".into());
    fe_color_matrix
        .attributes
        .insert("values".into(), format!("{amount}deg"));

    filter_elem.children.push(XMLNode::Element(fe_color_matrix));

    add_svg_filter(svg_elem, id, filter_elem)
}
