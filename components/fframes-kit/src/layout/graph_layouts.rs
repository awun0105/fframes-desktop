//! Where a graph's vertices go: networkx's layouts, as Manim's `Graph(layout=...)` names them.

use std::{collections::VecDeque, f64::consts::TAU};

use crate::motion::Rng;

use super::tidy_tree;

/// Positions of a graph's vertices in units, from its vertex count, its edges and the scale.
/// Plain `fn`, like Manim's `layout=` callable; the result is used as it is.
pub type LayoutFunction = fn(usize, &[(usize, usize)], f64) -> Vec<(f64, f64)>;

/// How [`graph_layout`] places the vertices of a graph (Manim `Graph(layout=...)`).
#[derive(Debug, Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
pub enum GraphLayout {
    /// On a circle, in order, from the right going counterclockwise.
    Circular,
    /// Graph distances become distances on the plane (stress majorization from a circle).
    KamadaKawai,
    /// `Spring` with seed 0: no planar embedding is computed, so crossings can stay.
    Planar,
    /// Anywhere in the square, from `seed`.
    Random { seed: u64 },
    /// On circles, the first list innermost (one vertex alone goes in the middle); vertices
    /// in no list go on the outer circle. No lists is one circle.
    Shell { shells: Vec<Vec<usize>> },
    /// The second and third eigenvectors of the graph's Laplacian. Above 400 vertices it is
    /// `Spring`.
    Spectral,
    /// Along a spiral from the middle, in order.
    Spiral,
    /// Edges pull, vertices push (Fruchterman-Reingold), from random starts by `seed`.
    Spring { seed: u64, iterations: usize },
    /// The lists in columns from left to right; vertices in no list form the last column.
    Partite { parts: Vec<Vec<usize>> },
    /// A tidy tree from `root` down, built breadth first over the edges; vertices not
    /// reached sit in a row below it.
    Tree { root: usize },
    /// Your own positions.
    Function(LayoutFunction),
}

impl Default for GraphLayout {
    /// Manim's default, `spring`.
    fn default() -> Self {
        Self::Spring {
            seed: 0,
            iterations: 50,
        }
    }
}

/// The positions of `count` vertices joined by `edges`, in units with `y` up, centered on
/// the origin and scaled so the farthest coordinate is `scale` (Manim's `layout_scale`, 2 by
/// default). Afterwards vertices closer than `0.4 * scale / sqrt(count)` are pushed apart.
/// Edges are taken both ways; self-loops and edges to missing vertices are left out. The same
/// arguments give the same positions on every run and machine.
pub fn graph_layout(
    layout: &GraphLayout,
    count: usize,
    edges: &[(usize, usize)],
    scale: f64,
) -> Vec<(f64, f64)> {
    let edges: Vec<(usize, usize)> = edges
        .iter()
        .copied()
        .filter(|&(a, b)| a != b && a < count && b < count)
        .collect();
    let raw = match layout {
        GraphLayout::Function(f) => {
            let mut points = f(count, &edges, scale);
            points.resize(count, (0.0, 0.0));
            return points;
        }
        GraphLayout::Circular => circle(count),
        GraphLayout::KamadaKawai => kamada_kawai(count, &edges),
        GraphLayout::Planar => spring(count, &edges, 0, 50),
        GraphLayout::Random { seed } => {
            let mut rng = Rng::new(*seed);
            (0..count).map(|_| (rng.unit(), rng.unit())).collect()
        }
        GraphLayout::Shell { shells } => shell(count, shells),
        GraphLayout::Spectral if count > 400 => spring(count, &edges, 0, 50),
        GraphLayout::Spectral => spectral(count, &edges),
        GraphLayout::Spiral => (0..count)
            .map(|i| {
                let (r, angle) = (i as f64, 0.35 * i as f64);
                (r * angle.cos(), r * angle.sin())
            })
            .collect(),
        GraphLayout::Spring { seed, iterations } => spring(count, &edges, *seed, *iterations),
        GraphLayout::Partite { parts } => partite(count, parts),
        GraphLayout::Tree { root } => tree(count, &edges, *root),
    };
    let mut points = rescale(raw, scale);
    if count > 1 {
        spread(&mut points, 0.4 * scale / (count as f64).sqrt());
    }
    points
}

/// Centers on the mean and scales so the largest coordinate is `scale` (networkx's
/// `rescale_layout`).
fn rescale(mut points: Vec<(f64, f64)>, scale: f64) -> Vec<(f64, f64)> {
    if points.is_empty() {
        return points;
    }
    let n = points.len() as f64;
    let mean = points
        .iter()
        .fold((0.0, 0.0), |(x, y), p| (x + p.0 / n, y + p.1 / n));
    let lim = points
        .iter()
        .map(|p| (p.0 - mean.0).abs().max((p.1 - mean.1).abs()))
        .fold(0.0, f64::max);
    let k = if lim > 0.0 { scale / lim } else { 0.0 };
    for p in &mut points {
        *p = ((p.0 - mean.0) * k, (p.1 - mean.1) * k);
    }
    points
}

/// Pushes vertices closer than `min` apart, in passes, by half of what is missing each.
/// Vertices on the same spot leave in a direction given by their index.
fn spread(points: &mut [(f64, f64)], min: f64) {
    for _ in 0..100 {
        let mut moved = false;
        for i in 0..points.len() {
            for j in i + 1..points.len() {
                let (dx, dy) = (points[j].0 - points[i].0, points[j].1 - points[i].1);
                let d = dx.hypot(dy);
                if d >= min {
                    continue;
                }
                let (ux, uy) = if d > 1e-9 {
                    (dx / d, dy / d)
                } else {
                    let a = j as f64 * 2.399_963;
                    (a.cos(), a.sin())
                };
                let push = (min - d) / 2.0 + 1e-9;
                points[i] = (points[i].0 - ux * push, points[i].1 - uy * push);
                points[j] = (points[j].0 + ux * push, points[j].1 + uy * push);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

fn neighbours(count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); count];
    for &(a, b) in edges {
        adj[a].push(b);
        adj[b].push(a);
    }
    adj
}

fn circle(count: usize) -> Vec<(f64, f64)> {
    if count == 1 {
        return vec![(0.0, 0.0)];
    }
    (0..count)
        .map(|i| {
            let a = TAU * i as f64 / count as f64;
            (a.cos(), a.sin())
        })
        .collect()
}

fn shell(count: usize, shells: &[Vec<usize>]) -> Vec<(f64, f64)> {
    let mut seen = vec![false; count];
    let mut lists: Vec<Vec<usize>> = shells
        .iter()
        .map(|s| {
            s.iter()
                .copied()
                .filter(|&v| v < count && !std::mem::replace(&mut seen[v], true))
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect();
    let rest: Vec<usize> = (0..count).filter(|&v| !seen[v]).collect();
    if !rest.is_empty() {
        lists.push(rest);
    }
    let mut points = vec![(0.0, 0.0); count];
    let bump = 1.0 / lists.len().max(1) as f64;
    // one vertex alone in the first shell sits in the middle
    let mut radius = if lists.first().is_some_and(|s| s.len() == 1) {
        0.0
    } else {
        bump
    };
    for (k, list) in lists.iter().enumerate() {
        let turn = k as f64 * std::f64::consts::PI / lists.len() as f64;
        for (i, &v) in list.iter().enumerate() {
            let a = turn + TAU * i as f64 / list.len() as f64;
            points[v] = (radius * a.cos(), radius * a.sin());
        }
        radius += bump;
    }
    points
}

fn partite(count: usize, parts: &[Vec<usize>]) -> Vec<(f64, f64)> {
    let mut seen = vec![false; count];
    let mut columns: Vec<Vec<usize>> = parts
        .iter()
        .map(|s| {
            s.iter()
                .copied()
                .filter(|&v| v < count && !std::mem::replace(&mut seen[v], true))
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect();
    let rest: Vec<usize> = (0..count).filter(|&v| !seen[v]).collect();
    if !rest.is_empty() {
        columns.push(rest);
    }
    let mut points = vec![(0.0, 0.0); count];
    for (x, column) in columns.iter().enumerate() {
        let middle = (column.len() as f64 - 1.0) / 2.0;
        for (i, &v) in column.iter().enumerate() {
            // the first of a column on top
            points[v] = (x as f64, middle - i as f64);
        }
    }
    points
}

fn tree(count: usize, edges: &[(usize, usize)], root: usize) -> Vec<(f64, f64)> {
    if root >= count {
        return circle(count);
    }
    let adj = neighbours(count, edges);
    let mut children = vec![Vec::new(); count];
    let mut seen = vec![false; count];
    seen[root] = true;
    let mut queue = VecDeque::from([root]);
    while let Some(u) = queue.pop_front() {
        for &v in &adj[u] {
            if !seen[v] {
                seen[v] = true;
                children[u].push(v);
                queue.push_back(v);
            }
        }
    }
    let slots = tidy_tree(&children, root);
    let depth = (0..count)
        .filter(|&v| seen[v])
        .map(|v| slots[v].1)
        .fold(0.0, f64::max);
    let mut next = 0.0;
    (0..count)
        .map(|v| {
            if seen[v] {
                (slots[v].0, -slots[v].1)
            } else {
                next += 1.0;
                (next - 1.0, -depth - 1.0)
            }
        })
        .collect()
}

/// Graph distances by breadth first search; vertices in other parts are `count` apart.
fn distances(count: usize, edges: &[(usize, usize)]) -> Vec<Vec<f64>> {
    let adj = neighbours(count, edges);
    (0..count)
        .map(|s| {
            let mut d = vec![count as f64; count];
            d[s] = 0.0;
            let mut queue = VecDeque::from([s]);
            while let Some(u) = queue.pop_front() {
                for &v in &adj[u] {
                    if d[v] > d[u] + 1.0 {
                        d[v] = d[u] + 1.0;
                        queue.push_back(v);
                    }
                }
            }
            d
        })
        .collect()
}

fn kamada_kawai(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    let d = distances(count, edges);
    let mut points: Vec<(f64, f64)> = circle(count)
        .into_iter()
        .map(|(x, y)| (x * count as f64 / TAU, y * count as f64 / TAU))
        .collect();
    // stress majorization, one vertex at a time: every vertex moves to where its distances
    // to the others, weighted by 1/d², fit best
    for _ in 0..300 {
        for i in 0..count {
            let (mut sx, mut sy, mut sw) = (0.0, 0.0, 0.0);
            for j in (0..count).filter(|&j| j != i) {
                let w = 1.0 / (d[i][j] * d[i][j]);
                let (dx, dy) = (points[i].0 - points[j].0, points[i].1 - points[j].1);
                let len = dx.hypot(dy).max(1e-9);
                sx += w * (points[j].0 + d[i][j] * dx / len);
                sy += w * (points[j].1 + d[i][j] * dy / len);
                sw += w;
            }
            if sw > 0.0 {
                points[i] = (sx / sw, sy / sw);
            }
        }
    }
    points
}

fn spring(count: usize, edges: &[(usize, usize)], seed: u64, iterations: usize) -> Vec<(f64, f64)> {
    let mut rng = Rng::new(seed);
    let mut points: Vec<(f64, f64)> = (0..count).map(|_| (rng.unit(), rng.unit())).collect();
    if count < 2 {
        return points;
    }
    let mut joined = vec![false; count * count];
    for &(a, b) in edges {
        joined[a * count + b] = true;
        joined[b * count + a] = true;
    }
    let k = (1.0 / count as f64).sqrt();
    let mut t = 0.1;
    let cool = t / (iterations as f64 + 1.0);
    for _ in 0..iterations {
        let moves: Vec<(f64, f64)> = (0..count)
            .map(|i| {
                let (mut mx, mut my) = (0.0, 0.0);
                for j in (0..count).filter(|&j| j != i) {
                    let (dx, dy) = (points[i].0 - points[j].0, points[i].1 - points[j].1);
                    let d = dx.hypot(dy).max(0.01);
                    let pull = if joined[i * count + j] { d / k } else { 0.0 };
                    let force = k * k / (d * d) - pull;
                    mx += dx * force;
                    my += dy * force;
                }
                let len = mx.hypot(my).max(0.01);
                (mx * t / len, my * t / len)
            })
            .collect();
        for (p, m) in points.iter_mut().zip(moves) {
            *p = (p.0 + m.0, p.1 + m.1);
        }
        t -= cool;
    }
    points
}

fn spectral(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    if count < 3 {
        return circle(count);
    }
    // the Laplacian, D - A
    let mut a = vec![0.0; count * count];
    for &(u, v) in edges {
        if a[u * count + v] == 0.0 {
            a[u * count + v] = -1.0;
            a[v * count + u] = -1.0;
            a[u * count + u] += 1.0;
            a[v * count + v] += 1.0;
        }
    }
    let (values, vectors) = jacobi(a, count);
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|&i, &j| values[i].total_cmp(&values[j]).then(i.cmp(&j)));
    let (e1, e2) = (order[1], order[2]);
    (0..count)
        .map(|v| (vectors[v * count + e1], vectors[v * count + e2]))
        .collect()
}

/// The eigenvalues and eigenvectors (as columns) of the symmetric `n` by `n` matrix `a`, by
/// cyclic Jacobi rotations.
fn jacobi(mut a: Vec<f64>, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    for _ in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i * n + j] * a[i * n + j])
            .sum();
        if off < 1e-18 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = a[p * n + q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k * n + p], a[k * n + q]);
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p * n + k], a[q * n + k]);
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i * n + i]).collect(), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ring of six with two chords and a tail of two.
    const EDGES: [(usize, usize); 10] = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 5),
        (5, 0),
        (0, 3),
        (1, 4),
        (5, 6),
        (6, 7),
    ];

    fn layouts() -> Vec<GraphLayout> {
        vec![
            GraphLayout::Circular,
            GraphLayout::KamadaKawai,
            GraphLayout::Planar,
            GraphLayout::Random { seed: 3 },
            GraphLayout::Shell {
                shells: vec![vec![0], vec![1, 2, 3, 4, 5]],
            },
            GraphLayout::Spectral,
            GraphLayout::Spiral,
            GraphLayout::default(),
            GraphLayout::Partite {
                parts: vec![vec![0, 2, 4], vec![1, 3, 5]],
            },
            GraphLayout::Tree { root: 0 },
        ]
    }

    #[test]
    fn every_layout_is_deterministic_and_keeps_vertices_apart() {
        let min = 0.4 * 2.0 / 8f64.sqrt();
        for layout in layouts() {
            let a = graph_layout(&layout, 8, &EDGES, 2.0);
            let b = graph_layout(&layout, 8, &EDGES, 2.0);
            let bits = |p: &[(f64, f64)]| -> Vec<(u64, u64)> {
                p.iter().map(|q| (q.0.to_bits(), q.1.to_bits())).collect()
            };
            assert_eq!(bits(&a), bits(&b), "{layout:?}");
            assert_eq!(a.len(), 8);
            for i in 0..8 {
                assert!(a[i].0.is_finite() && a[i].1.is_finite(), "{layout:?}");
                for j in i + 1..8 {
                    let d = (a[i].0 - a[j].0).hypot(a[i].1 - a[j].1);
                    assert!(d >= min * 0.99, "{layout:?} {i} {j} {d}");
                }
            }
            // within the scale, give or take the push apart
            let far = a
                .iter()
                .map(|p| p.0.abs().max(p.1.abs()))
                .fold(0.0, f64::max);
            assert!(far <= 2.0 + min, "{layout:?} {far}");
        }
    }

    #[test]
    fn layouts_place_what_they_promise() {
        let circle = graph_layout(&GraphLayout::Circular, 8, &EDGES, 2.0);
        for p in &circle {
            assert!((p.0.hypot(p.1) - 2.0).abs() < 1e-9);
        }
        // the tree hangs from its root, the tail two levels under the ring
        let tree = graph_layout(&GraphLayout::Tree { root: 0 }, 8, &EDGES, 2.0);
        assert!((1..8).all(|v| tree[v].1 < tree[0].1));
        assert!(tree[7].1 < tree[6].1 && tree[6].1 < tree[5].1);
        // the two parts are columns, left and right, and the rest a third
        let parts = graph_layout(&layouts()[8], 8, &EDGES, 2.0);
        assert!((parts[0].0 - parts[2].0).abs() < 1e-9 && parts[0].0 < parts[1].0);
        assert!(parts[1].0 < parts[6].0);
        // the alone vertex of the first shell is in the middle
        let shells = graph_layout(&layouts()[4], 8, &EDGES, 2.0);
        assert!(shells[0].0.hypot(shells[0].1) < shells[1].0.hypot(shells[1].1));
        // in Kamada-Kawai, a neighbour is nearer than a vertex three steps away
        let kk = graph_layout(&GraphLayout::KamadaKawai, 8, &EDGES, 2.0);
        let d = |a: usize, b: usize| (kk[a].0 - kk[b].0).hypot(kk[a].1 - kk[b].1);
        assert!(d(6, 7) < d(2, 7));
        // your own function is used as it is
        let own = graph_layout(
            &GraphLayout::Function(|n, _, s| (0..n).map(|i| (i as f64 * s, 0.0)).collect()),
            3,
            &[],
            2.0,
        );
        assert_eq!(own, vec![(0.0, 0.0), (2.0, 0.0), (4.0, 0.0)]);
    }
}
