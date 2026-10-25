//! Inserting, removing and replacing layers of a network over time (`ManimML`'s
//! `insert_layer`, `remove_layer` and `replace_layer` animations).

use kurbo::{Point, Rect};

use crate::motion::cubic_in_out;

use super::layers::{Layer, LayerGeometry};

/// A change to the layers of a [`NeuralNetwork`](super::NeuralNetwork). The index counts the
/// layers as the changes before it left them; an index out of range changes nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerChange<'a> {
    /// `layer` goes in at `index`, the layers from there on move right.
    Insert { index: usize, layer: Layer<'a> },
    /// The layer at `index` goes, the layers after it close the gap.
    Remove { index: usize },
    /// The layer at `index` fades out while `layer` fades in in its place.
    Replace { index: usize, layer: Layer<'a> },
}

/// A [`LayerChange`] that runs from `start` over `duration` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedChange<'a> {
    pub change: LayerChange<'a>,
    pub start: f32,
    pub duration: f32,
}

/// The layers after `changes`, each with its id: the layers given are `0..n`, the layer change
/// `k` brings in is `n + k`.
pub(super) fn apply<'l, 'a>(
    layers: &'l [Layer<'a>],
    changes: &'l [TimedChange<'a>],
) -> Vec<(usize, &'l Layer<'a>)> {
    let n = layers.len();
    let mut list: Vec<(usize, &Layer<'a>)> = layers.iter().enumerate().collect();
    for (k, c) in changes.iter().enumerate() {
        match &c.change {
            LayerChange::Insert { index, layer } => {
                if *index <= list.len() {
                    list.insert(*index, (n + k, layer));
                }
            }
            LayerChange::Remove { index } => {
                if *index < list.len() {
                    list.remove(*index);
                }
            }
            LayerChange::Replace { index, layer } => {
                if let Some(slot) = list.get_mut(*index) {
                    *slot = (n + k, layer);
                }
            }
        }
    }
    list
}

/// The change running at `now` and how far it is, eased from 0 to 1: the last change that has
/// started (the changes before it are done). `None` before the first one starts.
pub(super) fn running(changes: &[TimedChange<'_>], now: f32) -> Option<(usize, f64)> {
    let k = changes.iter().rposition(|c| c.start <= now)?;
    let c = &changes[k];
    let t = if c.duration > 0.0 {
        ((now - c.start) / c.duration).clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some((k, f64::from(cubic_in_out(t))))
}

fn lerp_rect(a: Rect, b: Rect, p: f64) -> Rect {
    Rect::new(
        a.x0 + (b.x0 - a.x0) * p,
        a.y0 + (b.y0 - a.y0) * p,
        a.x1 + (b.x1 - a.x1) * p,
        a.y1 + (b.y1 - a.y1) * p,
    )
}

/// `a` and `b` mixed element by element when they have the same length, else the nearer one.
fn lerp_all<T: Clone>(a: &[T], b: &[T], p: f64, lerp: impl Fn(&T, &T) -> T) -> Vec<T> {
    if a.len() == b.len() {
        a.iter().zip(b).map(|(x, y)| lerp(x, y)).collect()
    } else if p < 0.5 {
        a.to_vec()
    } else {
        b.to_vec()
    }
}

/// The geometry `p` (0 to 1) of the way from `a` to `b`. What cannot move (a different number
/// of neurons or maps, the cells of a map) cuts half way.
pub(super) fn lerp_geometry(a: &LayerGeometry, b: &LayerGeometry, p: f64) -> LayerGeometry {
    if p >= 1.0 {
        return b.clone();
    }
    let point = |x: &Point, y: &Point| x.lerp(*y, p);
    let near = if p < 0.5 { a } else { b };
    LayerGeometry {
        center: a.center.lerp(b.center, p),
        bounds: lerp_rect(a.bounds, b.bounds, p),
        nodes: lerp_all(&a.nodes, &b.nodes, p, point),
        maps: lerp_all(&a.maps, &b.maps, p, |x, y| {
            std::array::from_fn(|k| x[k].lerp(y[k], p))
        }),
        cells: near.cells,
        window: near.window,
        stride: near.stride,
        points: lerp_all(&a.points, &b.points, p, point),
        rows: lerp_all(&a.rows, &b.rows, p, point),
        images: lerp_all(&a.images, &b.images, p, |x, y| lerp_rect(*x, *y, p)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(change: LayerChange<'static>, start: f32) -> TimedChange<'static> {
        TimedChange {
            change,
            start,
            duration: 1.0,
        }
    }

    #[test]
    fn changes_give_new_layers_new_ids() {
        let layers = [
            Layer::feed_forward(2),
            Layer::feed_forward(3),
            Layer::feed_forward(4),
        ];
        let changes = [
            change(
                LayerChange::Insert {
                    index: 1,
                    layer: Layer::feed_forward(5),
                },
                0.0,
            ),
            change(LayerChange::Remove { index: 3 }, 1.0),
            change(
                LayerChange::Replace {
                    index: 0,
                    layer: Layer::feed_forward(6),
                },
                2.0,
            ),
            // out of range: nothing happens
            change(LayerChange::Remove { index: 9 }, 3.0),
        ];
        let ids = |k: usize| -> Vec<usize> {
            apply(&layers, &changes[..k])
                .iter()
                .map(|(id, _)| *id)
                .collect()
        };
        assert_eq!(ids(0), vec![0, 1, 2]);
        assert_eq!(ids(1), vec![0, 3, 1, 2]);
        assert_eq!(ids(2), vec![0, 3, 1]);
        assert_eq!(ids(3), vec![5, 3, 1]);
        assert_eq!(ids(4), ids(3));
        assert_eq!(apply(&layers, &changes[..1])[1].1, &Layer::feed_forward(5));
    }

    #[test]
    fn the_last_started_change_runs() {
        let changes = [
            change(LayerChange::Remove { index: 0 }, 1.0),
            change(LayerChange::Remove { index: 0 }, 3.0),
        ];
        assert_eq!(running(&changes, 0.5), None);
        assert_eq!(running(&changes, 1.0), Some((0, 0.0)));
        assert_eq!(running(&changes, 1.5), Some((0, 0.5)));
        assert_eq!(running(&changes, 2.5), Some((0, 1.0)));
        assert_eq!(running(&changes, 3.0), Some((1, 0.0)));
        assert_eq!(running(&changes, 9.0), Some((1, 1.0)));
    }

    #[test]
    fn geometry_moves_and_cuts_what_cannot_move() {
        let a = LayerGeometry {
            center: Point::new(0.0, 0.0),
            bounds: Rect::new(-10.0, -10.0, 10.0, 10.0),
            nodes: vec![Point::new(0.0, -5.0), Point::new(0.0, 5.0)],
            cells: 4,
            ..Default::default()
        };
        let b = LayerGeometry {
            center: Point::new(100.0, 0.0),
            bounds: Rect::new(90.0, -20.0, 110.0, 20.0),
            nodes: vec![Point::new(100.0, -15.0), Point::new(100.0, 15.0)],
            rows: vec![Point::new(100.0, 0.0)],
            cells: 8,
            ..Default::default()
        };
        let half = lerp_geometry(&a, &b, 0.5);
        assert_eq!(half.center, Point::new(50.0, 0.0));
        assert_eq!(half.bounds, Rect::new(40.0, -15.0, 60.0, 15.0));
        assert_eq!(
            half.nodes,
            vec![Point::new(50.0, -10.0), Point::new(50.0, 10.0)]
        );
        // one row against none, and the cells, cut half way
        assert_eq!(lerp_geometry(&a, &b, 0.4).rows, Vec::new());
        assert_eq!(half.rows, b.rows);
        assert_eq!((lerp_geometry(&a, &b, 0.4).cells, half.cells), (4, 8));
        assert_eq!(lerp_geometry(&a, &b, 0.0), a);
        assert_eq!(lerp_geometry(&a, &b, 1.0), b);
    }
}
