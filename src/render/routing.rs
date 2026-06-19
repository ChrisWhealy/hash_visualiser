//! Orthogonal ("elbow") wire routing.
//!
//! Straight edge-centre-to-edge-centre lines have two problems once the layout has more than a trivial shape: long
//! edges cut straight through the nodes between their endpoints, and several wires sharing a source or target overlap
//! into one indistinguishable line. This module routes each wire as an axis-aligned polyline instead:
//!
//! - **Fan-out**: a node's incoming/outgoing wires are spread along its edge (ordered by the opposite endpoint), so no
//!   two share a line where they meet the node.
//! - **Elbow**: a wire leaves its source, runs vertically in the inter-layer gap just before the target, then enters.
//!   For a one-layer hop the whole route stays inside the clear gap between the two columns.
//! - **Detour**: if that simple elbow would cross another node (a multi-layer "long" edge), the wire is rerouted out
//!   to a lane beyond the node band, so it travels over the top/side and never passes through a node.
//!
//! Everything is expressed in (main, cross) terms — main = the flow axis, cross = perpendicular — so the same logic
//! serves all four flow directions.

use std::collections::HashMap;

use svg_dom::root::utils::Point;

use crate::{
    ast::{ebnf_07::WireEndpoint, ebnf_08::FlowDirection},
    graph::ValidatedGraph,
    render::rect::Rect,
};

use super::layout::{LAYER_GAP, downstream, entry_point, exit_point, upstream};

/// How far beyond the node band a detour ("flyover") lane sits.
const LANE_CLEARANCE: f64 = 28.0;
/// Cross-axis separation between stacked detour lanes.
const LANE_STEP: f64 = 16.0;
/// Main-axis stagger between vertical channels that share an inter-layer gap, so they don't become collinear.
const CHANNEL_STEP: f64 = 12.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// (main, cross) helpers — main is the flow axis, cross is perpendicular.
fn m(p: Point, horizontal: bool) -> f64 {
    if horizontal { p.x } else { p.y }
}
fn c(p: Point, horizontal: bool) -> f64 {
    if horizontal { p.y } else { p.x }
}
fn pt(main: f64, cross: f64, horizontal: bool) -> Point {
    if horizontal {
        Point::new(main, cross)
    } else {
        Point::new(cross, main)
    }
}
fn cross_lo(r: &Rect, horizontal: bool) -> f64 {
    if horizontal { r.top_left.y } else { r.top_left.x }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// True if the axis-aligned segment `a`–`b` passes through the interior of `r` (touching an edge does not count).
fn segment_hits_rect(a: Point, b: Point, r: &Rect) -> bool {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    let (rx0, ry0) = (r.top_left.x, r.top_left.y);
    let (rx1, ry1) = (rx0 + r.size.width, ry0 + r.size.height);
    x1 > rx0 && x0 < rx1 && y1 > ry0 && y0 < ry1
}

/// True if any segment of `points` passes through any rect in `obstacles`.
fn route_hits_any(points: &[Point], obstacles: &[Rect]) -> bool {
    points
        .windows(2)
        .any(|seg| obstacles.iter().any(|r| segment_hits_rect(seg[0], seg[1], r)))
}

/// The node name behind a wire endpoint, or `None` for an open (`?`) endpoint.
fn node_name(ep: &WireEndpoint) -> Option<&str> {
    match ep {
        WireEndpoint::Node(n) => Some(n.as_str()),
        WireEndpoint::Open => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Routes one wire from `exit` to `entry`. The simple elbow turns in the gap at `channel_m`; if it would cross an
/// obstacle, the wire detours out to `lane_c` (a cross position beyond every node) and travels there instead.
fn route(
    exit: Point,
    entry: Point,
    horizontal: bool,
    channel_m: f64,
    lane_c: f64,
    obstacles: &[Rect],
) -> Vec<Point> {
    let simple = vec![
        exit,
        pt(channel_m, c(exit, horizontal), horizontal),
        pt(channel_m, c(entry, horizontal), horizontal),
        entry,
    ];
    if !route_hits_any(&simple, obstacles) {
        return simple;
    }

    // The simple elbow would clip a node: detour over the lane. Step into the gap after the source, jog out to the
    // lane, run along it (clear of every node), then drop into the gap before the target and enter.
    let dir = (m(entry, horizontal) - m(exit, horizontal)).signum();
    let stub_m = m(exit, horizontal) + dir * LAYER_GAP * 0.5;
    vec![
        exit,
        pt(stub_m, c(exit, horizontal), horizontal),
        pt(stub_m, lane_c, horizontal),
        pt(channel_m, lane_c, horizontal),
        pt(channel_m, c(entry, horizontal), horizontal),
        entry,
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Computes an orthogonal polyline for every wire, aligned with `graph.wires`. `None` marks a wire with two open
/// endpoints (nothing to draw). Open-ended wires keep a short straight stub.
///
/// `conn` gives each node's wire-attachment anchor on the cross axis as `(centre, extent)` — the centre and span of the
/// node's *visible body* (e.g. the value cell, below a register's label band), so wires meet the body rather than the
/// full footprint.
pub(super) fn route_all(
    graph: &ValidatedGraph,
    placement: &HashMap<String, Rect>,
    conn: &HashMap<String, (f64, f64)>,
) -> Vec<Option<Vec<Point>>> {
    let flow = &graph.flow;
    let horizontal = matches!(flow, FlowDirection::LeftToRight | FlowDirection::RightToLeft);

    let rect_of = |ep: &WireEndpoint| match ep {
        WireEndpoint::Node(n) => placement.get(n),
        WireEndpoint::Open => None,
    };
    let cross_centre_of = |name: &str| conn.get(name).map(|&(centre, _)| centre).unwrap_or(0.0);

    // Group concrete (node→node) wires by their source and target, to fan their endpoints out along each node's edge.
    let mut outgoing: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut incoming: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in graph.wires.iter().enumerate() {
        if let (Some(s), Some(t)) = (node_name(&w.source), node_name(&w.target)) {
            outgoing.entry(s).or_default().push(i);
            incoming.entry(t).or_default().push(i);
        }
    }
    // Order each node's wires by the cross position of the opposite endpoint (fewer crossings).
    for v in outgoing.values_mut() {
        v.sort_by(|&a, &b| {
            let (na, nb) = (node_name(&graph.wires[a].target), node_name(&graph.wires[b].target));
            cross_centre_of(na.unwrap_or("")).total_cmp(&cross_centre_of(nb.unwrap_or("")))
        });
    }
    for v in incoming.values_mut() {
        v.sort_by(|&a, &b| {
            let (na, nb) = (node_name(&graph.wires[a].source), node_name(&graph.wires[b].source));
            cross_centre_of(na.unwrap_or("")).total_cmp(&cross_centre_of(nb.unwrap_or("")))
        });
    }

    // Assign each wire a fanned-out exit/entry cross position, and remember its slot among the target's inputs.
    let mut exit_cross: HashMap<usize, f64> = HashMap::new();
    let mut entry_cross: HashMap<usize, f64> = HashMap::new();
    let mut in_slot: HashMap<usize, (usize, usize)> = HashMap::new();
    let fan = |lo: f64, size: f64, k: usize, n: usize| lo + (k as f64 + 1.0) / (n as f64 + 1.0) * size;

    for (name, wires) in &outgoing {
        let (centre, extent) = conn[*name];
        let lo = centre - extent / 2.0;
        for (k, &i) in wires.iter().enumerate() {
            exit_cross.insert(i, fan(lo, extent, k, wires.len()));
        }
    }
    for (name, wires) in &incoming {
        let (centre, extent) = conn[*name];
        let lo = centre - extent / 2.0;
        for (k, &i) in wires.iter().enumerate() {
            entry_cross.insert(i, fan(lo, extent, k, wires.len()));
            in_slot.insert(i, (k, wires.len()));
        }
    }

    let cross_min = placement
        .values()
        .map(|r| cross_lo(r, horizontal))
        .fold(f64::INFINITY, f64::min);

    let mut routes = Vec::with_capacity(graph.wires.len());
    let mut lane = 0usize;

    for (i, w) in graph.wires.iter().enumerate() {
        let route = match (rect_of(&w.source), rect_of(&w.target)) {
            (Some(s), Some(d)) => {
                let exit_m = m(exit_point(s, flow), horizontal);
                let entry_m = m(entry_point(d, flow), horizontal);
                let exit = pt(exit_m, exit_cross[&i], horizontal);
                let entry = pt(entry_m, entry_cross[&i], horizontal);

                let dir = (entry_m - exit_m).signum();
                let (slot, count) = in_slot[&i];
                let stagger = (slot as f64 - (count as f64 - 1.0) / 2.0) * CHANNEL_STEP;
                let channel_m = entry_m - dir * LAYER_GAP * 0.5 + stagger;

                let src = node_name(&w.source);
                let dst = node_name(&w.target);
                let obstacles: Vec<Rect> = placement
                    .iter()
                    .filter(|(n, _)| Some(n.as_str()) != src && Some(n.as_str()) != dst)
                    .map(|(_, r)| *r)
                    .collect();

                // The lane index only advances when a detour is actually used, so flyovers stack tightly.
                let lane_c = cross_min - LANE_CLEARANCE - lane as f64 * LANE_STEP;
                let points = route(exit, entry, horizontal, channel_m, lane_c, &obstacles);
                if points.len() > 4 {
                    lane += 1;
                }
                Some(points)
            }
            (Some(s), None) => {
                let start = exit_point(s, flow);
                Some(vec![start, downstream(start, flow)])
            }
            (None, Some(d)) => {
                let end = entry_point(d, flow);
                Some(vec![upstream(end, flow), end])
            }
            (None, None) => None,
        };
        routes.push(route);
    }

    routes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod tests {
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
}
