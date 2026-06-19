use super::*;
use svg_dom::root::utils::Size;

fn orthogonal(points: &[Point]) -> bool {
    points
        .windows(2)
        .all(|s| (s[0].x - s[1].x).abs() < 1e-9 || (s[0].y - s[1].y).abs() < 1e-9)
}

#[test]
fn should_route_a_short_hop_as_a_clean_elbow() -> Result<(), String> {
    // Source right edge → target left edge, one gap apart, nothing in the way.
    let exit = Point::new(160.0, 70.0);
    let entry = Point::new(280.0, 130.0);
    let points = route(exit, entry, true, 220.0, 0.0, &[]);

    if points.len() != 4 {
        return Err(format!("expected a 4-point elbow, got {points:?}"));
    }
    if !orthogonal(&points) {
        return Err("elbow is not axis-aligned".into());
    }
    Ok(())
}

#[test]
fn should_detour_around_an_intervening_node() -> Result<(), String> {
    // A long edge whose straight elbow would cross a node sitting between the columns.
    let exit = Point::new(160.0, 70.0);
    let entry = Point::new(520.0, 210.0);
    let obstacle = Rect::new(Point::new(300.0, 40.0), Size::new(120.0, 200.0));

    // The simple elbow (turning near the target) must be detected as crossing it...
    let simple = route(exit, entry, true, 460.0, 0.0, &[]);
    if !route_hits_any(&simple, &[obstacle]) {
        return Err("test setup wrong: simple elbow should cross the obstacle".into());
    }

    // ...and the routed wire must avoid it, over a lane above the band (cross = 0).
    let points = route(exit, entry, true, 460.0, 0.0, &[obstacle]);
    if !orthogonal(&points) {
        return Err("detour is not axis-aligned".into());
    }
    if route_hits_any(&points, &[obstacle]) {
        return Err(format!("detour still crosses the node: {points:?}"));
    }
    Ok(())
}

#[test]
fn should_detect_only_interior_crossings() -> Result<(), String> {
    let r = Rect::new(Point::new(100.0, 100.0), Size::new(50.0, 50.0));
    // Through the middle: hit.
    if !segment_hits_rect(Point::new(80.0, 125.0), Point::new(200.0, 125.0), &r) {
        return Err("a segment through the interior should hit".into());
    }
    // Running along the top edge: not a hit.
    if segment_hits_rect(Point::new(80.0, 100.0), Point::new(200.0, 100.0), &r) {
        return Err("a segment along the edge should not hit".into());
    }
    Ok(())
}
