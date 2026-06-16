use super::*;
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §8  Group and layout declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_group_with_contains() -> Result<(), String> {
    let g = make_parser("group g { contains: [a, b, c] }")?
        .parse_group_decl()
        .map_err(|e| e.to_string())?;
    eq(g.name.as_str(), "g")?;
    match &g.items[0] {
        GroupItem::Contains(names) => {
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            eq(refs, vec!["a", "b", "c"])
        }
        other => Err(format!("expected Contains item, got {other:?}")),
    }
}

#[test]
fn should_allow_trailing_comma_in_group_contains() -> Result<(), String> {
    let g = make_parser("group g { contains: [a, b,] }")?
        .parse_group_decl()
        .map_err(|e| e.to_string())?;
    match &g.items[0] {
        GroupItem::Contains(names) => eq(names.len(), 2),
        other => Err(format!("expected Contains item, got {other:?}")),
    }
}

#[test]
fn should_parse_group_with_arrange_grid() -> Result<(), String> {
    let g = make_parser("group g { arrange: grid }")?
        .parse_group_decl()
        .map_err(|e| e.to_string())?;
    match &g.items[0] {
        GroupItem::Arrange(ArrangeMode::Grid) => Ok(()),
        other => Err(format!("expected Arrange(Grid), got {other:?}")),
    }
}

#[test]
fn should_parse_group_with_arrange_horizontal() -> Result<(), String> {
    let g = make_parser("group g { arrange: horizontal }")?
        .parse_group_decl()
        .map_err(|e| e.to_string())?;
    match &g.items[0] {
        GroupItem::Arrange(ArrangeMode::Horizontal) => Ok(()),
        other => Err(format!("expected Arrange(Horizontal), got {other:?}")),
    }
}

#[test]
fn should_parse_group_with_arrange_vertical() -> Result<(), String> {
    let g = make_parser("group g { arrange: vertical }")?
        .parse_group_decl()
        .map_err(|e| e.to_string())?;
    match &g.items[0] {
        GroupItem::Arrange(ArrangeMode::Vertical) => Ok(()),
        other => Err(format!("expected Arrange(Vertical), got {other:?}")),
    }
}

#[test]
fn should_parse_layout_left_to_right() -> Result<(), String> {
    eq(
        make_parser("layout: left_to_right")?
            .parse_layout_decl()
            .map_err(|e| e.to_string())?,
        FlowDirection::LeftToRight,
    )
}

#[test]
fn should_parse_layout_top_to_bottom() -> Result<(), String> {
    eq(
        make_parser("layout: top_to_bottom")?
            .parse_layout_decl()
            .map_err(|e| e.to_string())?,
        FlowDirection::TopToBottom,
    )
}

#[test]
fn should_parse_layout_right_to_left() -> Result<(), String> {
    eq(
        make_parser("layout: right_to_left")?
            .parse_layout_decl()
            .map_err(|e| e.to_string())?,
        FlowDirection::RightToLeft,
    )
}

#[test]
fn should_parse_layout_bottom_to_top() -> Result<(), String> {
    eq(
        make_parser("layout: bottom_to_top")?
            .parse_layout_decl()
            .map_err(|e| e.to_string())?,
        FlowDirection::BottomToTop,
    )
}

#[test]
fn should_error_on_invalid_flow_direction() -> Result<(), String> {
    let err = make_parser("layout: diagonal")?
        .parse_layout_decl()
        .unwrap_err();
    msg_contains(&err.message, "flow direction")
}

#[test]
fn should_error_on_invalid_arrange_mode() -> Result<(), String> {
    let err = make_parser("group g { arrange: diagonal }")?
        .parse_group_decl()
        .unwrap_err();
    msg_contains(&err.message, "arrange mode")
}

#[test]
fn should_error_on_unknown_group_item_keyword() -> Result<(), String> {
    let err = make_parser("group g { emit: foo }")?
        .parse_group_decl()
        .unwrap_err();
    if err.message.contains("contains") || err.message.contains("arrange") {
        Ok(())
    } else {
        Err(format!(
            "expected error mentioning `contains` or `arrange`, got: {}",
            err.message
        ))
    }
}
