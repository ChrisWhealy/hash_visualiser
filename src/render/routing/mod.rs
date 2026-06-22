//! Orthogonal ("elbow") wire routing.
//!
//! Straight edge-centre-to-edge-centre lines are suitable only for the most basic of layouts.
//! For more complex layouts (I.E. layouts that you would use in most real-life scenarios), there are two problems:
//! 
//! 1. Long edges are likely to cut straight through any nodes between their endpoints
//! 1. Several wires sharing a source or target overlap into one indistinguishable line.
//! 
//! This module is designe to route each wire as an axis-aligned polyline instead:
//!
//! - **Fan-out**: a node's incoming/outgoing wires are spread along its edge (ordered by the opposite endpoint), so no
//!   two share a line where they meet the node.
//! - **Elbow**: a wire leaves its source, runs vertically in the inter-layer gap just before the target, then enters.
//!   For a one-layer hop the whole route stays inside the clear gap between the two columns.
//! - **Detour**: if that simple elbow would cross another node (a multi-layer "long" edge), the wire is rerouted out
//!   to a lane beyond the node band, so it travels over the top/side and never passes through a node.
//!
//! Everything is expressed in (main, cross) terms:
//! 
//! * main = the flow axis
//! * cross = perpendicular to the flow axis
//! 
//! So the same logic applies irrespective of the flow direction

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
const LANE_STEP: f64 = 20.0;
/// Main-axis stagger between vertical channels that share an inter-layer gap, so they don't become collinear.
const CHANNEL_STEP: f64 = 18.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// (main, cross) helpers — main is the flow axis, cross is perpendicular.
fn m(p: Point, horizontal: bool) -> f64 {
    if horizontal { p.x } else { p.y }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn c(p: Point, horizontal: bool) -> f64 {
    if horizontal { p.y } else { p.x }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn pt(main: f64, cross: f64, horizontal: bool) -> Point {
    if horizontal {
        Point::new(main, cross)
    } else {
        Point::new(cross, main)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// True if any segment of `points` passes through any rect in `obstacles`.
fn route_hits_any(points: &[Point], obstacles: &[Rect]) -> bool {
    points
        .windows(2)
        .any(|seg| obstacles.iter().any(|r| segment_hits_rect(seg[0], seg[1], r)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The node name behind a wire endpoint, or `None` for an open (`?`) endpoint.
fn node_name(ep: &WireEndpoint) -> Option<&str> {
    match ep {
        WireEndpoint::Node(n) => Some(n.as_str()),
        WireEndpoint::Open => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of the simple elbow from `exit` to `entry`, turning in the gap at `channel_m`.
fn simple_elbow(exit: Point, entry: Point, horizontal: bool, channel_m: f64) -> Vec<Point> {
    vec![
        exit,
        pt(channel_m, c(exit, horizontal), horizontal),
        pt(channel_m, c(entry, horizontal), horizontal),
        entry,
    ]
}

/// Whether the simple elbow would cross an obstacle — i.e. the wire will need to detour. Used both when routing and,
/// up front, when ordering a node's ports (a detoured wire arrives from the lane side, not from its source's side).
fn would_detour(exit: Point, entry: Point, horizontal: bool, channel_m: f64, obstacles: &[Rect]) -> bool {
    route_hits_any(&simple_elbow(exit, entry, horizontal, channel_m), obstacles)
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
    if !would_detour(exit, entry, horizontal, channel_m, obstacles) {
        return simple_elbow(exit, entry, horizontal, channel_m);
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
/// For each wire (by index), check whether its simple elbow would cross a node — i.e. does it need to detour.
/// Computed with un-fanned, body-centred endpoints, purely to inform port ordering.
fn detour_flags(
    graph: &ValidatedGraph,
    placement: &HashMap<String, Rect>,
    conn: &HashMap<String, (f64, f64)>,
    horizontal: bool,
    flow: &FlowDirection,
) -> HashMap<usize, bool> {
    let centre = |name: &str| conn.get(name).map(|&(c, _)| c).unwrap_or(0.0);
    let mut flags = HashMap::new();

    for (i, w) in graph.wires.iter().enumerate() {
        let (WireEndpoint::Node(s), WireEndpoint::Node(t)) = (&w.source, &w.target) else {
            continue;
        };
        let (Some(sr), Some(tr)) = (placement.get(s), placement.get(t)) else {
            continue;
        };

        let exit_m = m(exit_point(sr, flow), horizontal);
        let entry_m = m(entry_point(tr, flow), horizontal);
        let exit = pt(exit_m, centre(s), horizontal);
        let entry = pt(entry_m, centre(t), horizontal);
        let channel_m = entry_m - (entry_m - exit_m).signum() * LAYER_GAP * 0.5;

        let obstacles: Vec<Rect> = placement
            .iter()
            .filter(|(n, _)| *n != s && *n != t)
            .map(|(_, r)| *r)
            .collect();

        flags.insert(i, would_detour(exit, entry, horizontal, channel_m, &obstacles));
    }

    flags
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Assigns each wire a `(track, track_count)` within its channel - the gap before its target's row.
/// A left-edge sweep packs legs onto as few tracks as possible: legs whose cross-spans overlap land on different
/// tracks, clear legs share one (so a pair that can't collide stays on a single, aligned track).
/// Track 0 is closest to the row; wires are processed left-to-right so a detoured leg (arriving from the lane, far on
/// the low-cross side) takes it.
fn assign_channel_tracks(
    graph: &ValidatedGraph,
    placement: &HashMap<String, Rect>,
    exit_cross: &HashMap<usize, f64>,
    entry_cross: &HashMap<usize, f64>,
    detoured: &HashMap<usize, bool>,
    cross_min: f64,
    flow: &FlowDirection,
) -> HashMap<usize, (usize, usize)> {
    let horizontal = matches!(flow, FlowDirection::LeftToRight | FlowDirection::RightToLeft);
    let lane_start = cross_min - LANE_CLEARANCE; // a detoured leg arrives from here (the lane), so it spans from the side
    let leg = |i: usize| -> (f64, f64) {
        let end = entry_cross[&i];
        let start = if *detoured.get(&i).unwrap_or(&false) {
            lane_start
        } else {
            exit_cross[&i]
        };
        (start.min(end), start.max(end))
    };

    // Group wires by channel: their target's entry edge along the main axis (identical for a whole row).
    let mut channels: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, w) in graph.wires.iter().enumerate() {
        if let (WireEndpoint::Node(_), WireEndpoint::Node(t)) = (&w.source, &w.target)
            && let Some(tr) = placement.get(t)
        {
            let entry_m = m(entry_point(tr, flow), horizontal);
            channels.entry(entry_m.round() as i64).or_default().push(i);
        }
    }

    let mut tracks: HashMap<usize, (usize, usize)> = HashMap::new();
    for wires in channels.values() {
        let mut order = wires.clone();
        order.sort_by(|&a, &b| leg(a).0.total_cmp(&leg(b).0));

        let mut track_ends: Vec<f64> = Vec::new(); // cross-end of the last leg placed on each track
        let mut assigned: Vec<(usize, usize)> = Vec::with_capacity(order.len());
        for &i in &order {
            let (lo, hi) = leg(i);
            let t = match track_ends.iter().position(|&end| end <= lo - 1.0) {
                Some(t) => {
                    track_ends[t] = hi;
                    t
                }
                None => {
                    track_ends.push(hi);
                    track_ends.len() - 1
                }
            };
            assigned.push((i, t));
        }

        let count = track_ends.len();
        for (i, t) in assigned {
            tracks.insert(i, (t, count));
        }
    }

    tracks
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Computes an orthogonal polyline for every wire, aligned with `graph.wires`.
/// `None` marks a wire with two open endpoints (nothing to draw).
/// Open-ended wires keep a short straight stub.
///
/// `conn` gives each node's wire-attachment anchor on the cross axis as `(centre, extent)` - the centre and span of the
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

    let cross_min = placement
        .values()
        .map(|r| cross_lo(r, horizontal))
        .fold(f64::INFINITY, f64::min);

    // Group concrete (node→node) wires by their source and target, to fan their endpoints out along each node's edge.
    let mut outgoing: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut incoming: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in graph.wires.iter().enumerate() {
        if let (Some(s), Some(t)) = (node_name(&w.source), node_name(&w.target)) {
            outgoing.entry(s).or_default().push(i);
            incoming.entry(t).or_default().push(i);
        }
    }

    // Decide up front which wires will need a detour (using un-fanned, centred endpoints), so a node's ports can be
    // ordered with that in mind: a detoured wire arrives from the lane side, so its source position is misleading.
    let detoured = detour_flags(graph, placement, conn, horizontal, flow);
    let rank = |i: usize| if *detoured.get(&i).unwrap_or(&false) { 0u8 } else { 1u8 };

    // Order each node's wires: detoured wires first (they enter from the lane side), then the rest by the cross
    // position of their opposite endpoint — which keeps direct wires from crossing.
    for v in outgoing.values_mut() {
        v.sort_by(|&a, &b| {
            let ka = (rank(a), cross_centre_of(node_name(&graph.wires[a].target).unwrap_or("")));
            let kb = (rank(b), cross_centre_of(node_name(&graph.wires[b].target).unwrap_or("")));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
    }
    for v in incoming.values_mut() {
        v.sort_by(|&a, &b| {
            let ka = (rank(a), cross_centre_of(node_name(&graph.wires[a].source).unwrap_or("")));
            let kb = (rank(b), cross_centre_of(node_name(&graph.wires[b].source).unwrap_or("")));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
    }

    // Fan each wire's exit/entry to a distinct position along its source/target edge.
    let mut exit_cross: HashMap<usize, f64> = HashMap::new();
    let mut entry_cross: HashMap<usize, f64> = HashMap::new();
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
        }
    }

    // Assign each wire's horizontal leg to a track within its channel (the gap before its target row): legs that
    // overlap get separate tracks, clear legs share one. Track 0 sits closest to the row.
    let channel_track =
        assign_channel_tracks(graph, placement, &exit_cross, &entry_cross, &detoured, cross_min, flow);

    let mut routes = Vec::with_capacity(graph.wires.len());
    let mut lane = 0usize;

    for (i, w) in graph.wires.iter().enumerate() {
        let route = match (rect_of(&w.source), rect_of(&w.target)) {
            (Some(s), Some(d)) => {
                let exit_m = m(exit_point(s, flow), horizontal);
                let entry_m = m(entry_point(d, flow), horizontal);
                let exit = pt(exit_m, exit_cross[&i], horizontal);
                let entry = pt(entry_m, entry_cross[&i], horizontal);

                let src = node_name(&w.source);
                let dst = node_name(&w.target);

                let dir = (entry_m - exit_m).signum();
                // Track 0 turns closest to the row; higher tracks step back toward the source. A single-track channel
                // gives stagger 0 (legs aligned). `dir` keeps "closest to the row" correct for every flow direction.
                let (track, tracks) = channel_track[&i];
                let stagger = dir * ((tracks as f64 - 1.0) / 2.0 - track as f64) * CHANNEL_STEP;
                let channel_m = entry_m - dir * LAYER_GAP * 0.5 + stagger;
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
mod unit_tests;
