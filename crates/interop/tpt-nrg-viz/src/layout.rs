//! Deterministic bus placement for the single-line diagram.
//!
//! A general force-directed layout is overkill for a power-system diagram and
//! would make the output non-reproducible. Instead buses are placed on a
//! coarse grid ordered by a breadth-first walk from the slack bus, which keeps
//! the slack on the left and the network flowing to the right, the way a
//! one-line diagram is normally read.

use std::collections::HashMap;

use tpt_nrg_core::{BusType, EnergySystem};

/// A point in SVG user-space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal position.
    pub x: f64,
    /// Vertical position.
    pub y: f64,
}

impl Point {
    /// Construct a point.
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Bus positions for a whole system, plus the canvas size they occupy.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Position of each bus, keyed by bus id.
    pub positions: HashMap<usize, Point>,
    /// Width of the drawing area.
    pub width: f64,
    /// Height of the drawing area.
    pub height: f64,
}

impl Layout {
    /// Position of a bus, or the origin when the bus is unknown.
    #[must_use]
    pub fn position(&self, bus: usize) -> Point {
        self.positions
            .get(&bus)
            .copied()
            .unwrap_or(Point::new(0.0, 0.0))
    }
}

/// Spacing between bus columns and rows, in user-space units.
const COLUMN_SPACING: f64 = 120.0;
/// Vertical spacing between buses in the same column.
const ROW_SPACING: f64 = 80.0;
/// Margin around the drawing area.
const MARGIN: f64 = 50.0;

/// Convert a count to `f64` for a coordinate.
///
/// Exact for magnitudes up to 2^53, far beyond the number of buses any
/// diagram can show, so the precision-loss allowance lives at this single
/// documented point.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn count_to_f64(value: usize) -> f64 {
    value as f64
}

/// Compute a deterministic layout for `system`.
///
/// Buses are assigned a depth equal to their shortest path from the slack bus
/// (or from the lowest-numbered bus when there is no slack), then sorted by
/// `(depth, id)` and filled into a grid column by column. Buses that are not
/// connected to the reference bus are placed after them, at increasing depth,
/// so no bus is ever dropped.
#[must_use]
pub fn compute(system: &EnergySystem) -> Layout {
    let ids: Vec<usize> = system.buses.iter().map(|b| b.id).collect();
    if ids.is_empty() {
        return Layout {
            positions: HashMap::new(),
            width: 2.0 * MARGIN,
            height: 2.0 * MARGIN,
        };
    }
    let reference = reference_bus(system);
    let depths = shortest_path_depths(system, reference);
    let mut ordered: Vec<(usize, usize)> = ids
        .iter()
        .map(|id| (depths.get(id).copied().unwrap_or(usize::MAX), *id))
        .collect();
    ordered.sort_unstable();

    let per_column = per_column_capacity(ordered.len());
    let mut positions = HashMap::with_capacity(ids.len());
    let mut filled = vec![0_usize; ordered.len().div_ceil(per_column).max(1)];
    let mut max_rows = 0_usize;
    for (index, (_, id)) in ordered.iter().enumerate() {
        let col = index / per_column;
        let row = filled[col];
        filled[col] += 1;
        max_rows = max_rows.max(row + 1);
        positions.insert(
            *id,
            Point::new(
                MARGIN + count_to_f64(col) * COLUMN_SPACING,
                MARGIN + count_to_f64(row) * ROW_SPACING,
            ),
        );
    }
    let columns = filled.len().max(1);
    Layout {
        positions,
        width: 2.0 * MARGIN + count_to_f64(columns.saturating_sub(1)) * COLUMN_SPACING,
        height: 2.0 * MARGIN + count_to_f64(max_rows.saturating_sub(1)) * ROW_SPACING,
    }
}

/// Number of buses to place in each column.
///
/// Aiming for a roughly 4:3 aspect ratio keeps the diagram readable for both
/// wide (a transmission chain) and tall (a radial feeder) networks.
fn per_column_capacity(n: usize) -> usize {
    n.div_ceil(4).max(1)
}

/// The bus the layout grows from: the slack bus, or the lowest bus id.
fn reference_bus(system: &EnergySystem) -> usize {
    system
        .buses
        .iter()
        .find(|b| b.bus_type == BusType::Slack)
        .or_else(|| system.buses.first())
        .map_or(0, |b| b.id)
}

/// Shortest-path distance in bus count from `reference` to every reachable bus.
///
/// Unreachable buses are absent from the map and are given the highest depth by
/// the caller, so the placement is total.
fn shortest_path_depths(system: &EnergySystem, reference: usize) -> HashMap<usize, usize> {
    let index: HashMap<usize, usize> = system
        .buses
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); system.buses.len()];
    for br in &system.branches {
        if !br.in_service {
            continue;
        }
        if let (Some(&a), Some(&b)) = (index.get(&br.from_bus), index.get(&br.to_bus)) {
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
    }

    let mut depths = HashMap::new();
    let Some(&start) = index.get(&reference) else {
        return depths;
    };
    let mut queue = std::collections::VecDeque::from([(start, 0_usize)]);
    depths.insert(reference, 0);
    while let Some((node, depth)) = queue.pop_front() {
        for &next in &adjacency[node] {
            let id = system.buses[next].id;
            // Only the first (shortest) visit may set the depth; a later visit
            // must not overwrite it, or the reference bus would end up with a
            // longer path than its own neighbours.
            if let std::collections::hash_map::Entry::Vacant(slot) = depths.entry(id) {
                slot.insert(depth + 1);
                queue.push_back((next, depth + 1));
            }
        }
    }
    depths
}
