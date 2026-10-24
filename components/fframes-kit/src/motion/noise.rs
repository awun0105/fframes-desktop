//! Simplex noise in two, three and four dimensions (Remotion's `@remotion/noise`): smooth
//! values from -1 to 1 for driving motion, the same for the same seed and coordinates on
//! every machine.

/// A seed for [`noise_2d`], [`noise_3d`] and [`noise_4d`]: a number, or a string as in
/// Remotion (`noise2D("my-seed", x, y)`), hashed with FNV-1a.
pub trait NoiseSeed {
    fn noise_seed(&self) -> u64;
}

impl NoiseSeed for &str {
    fn noise_seed(&self) -> u64 {
        self.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
}

impl NoiseSeed for String {
    fn noise_seed(&self) -> u64 {
        self.as_str().noise_seed()
    }
}

impl NoiseSeed for u64 {
    fn noise_seed(&self) -> u64 {
        *self
    }
}

impl NoiseSeed for u32 {
    fn noise_seed(&self) -> u64 {
        u64::from(*self)
    }
}

/// Negative seeds wrap round, so each one still picks its own pattern.
impl NoiseSeed for i32 {
    fn noise_seed(&self) -> u64 {
        i64::from(*self) as u64
    }
}

impl NoiseSeed for i64 {
    fn noise_seed(&self) -> u64 {
        *self as u64
    }
}

impl NoiseSeed for usize {
    fn noise_seed(&self) -> u64 {
        *self as u64
    }
}

/// A random index below `n` for the lattice corner `(i, j, k, l)`.
fn corner(seed: u64, cell: [i64; 4], n: usize) -> usize {
    let mut h = seed ^ 0x9e37_79b9_7f4a_7c15;
    for v in cell {
        h = (h ^ v as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h ^= h >> 31;
    }
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^= h >> 29;
    (h % n as u64) as usize
}

/// The gradients of the 2D and 3D noise: the midpoints of a cube's edges.
const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// The gradients of the 4D noise: every vector with one 0 and three ±1.
fn grad4(index: usize) -> [f64; 4] {
    let zero = index / 8;
    let signs = index % 8;
    let mut g = [0.0; 4];
    let mut bit = 0;
    for (axis, v) in g.iter_mut().enumerate() {
        if axis != zero {
            *v = if signs >> bit & 1 == 1 { -1.0 } else { 1.0 };
            bit += 1;
        }
    }
    g
}

/// 2D simplex noise at `(x, y)`, from -1 to 1 (Remotion's `noise2D`). Features are about one
/// unit apart: divide the coordinates to make the noise broader.
///
/// ```
/// use fframes_kit::motion::noise_2d;
///
/// // a wobble that never repeats: the y offset of a dot over time
/// let dy = |t: f32| 30.0 * noise_2d("dot", 0.0, t * 0.8);
/// assert_eq!(dy(1.25), dy(1.25));
/// assert!(dy(1.25).abs() <= 30.0);
/// ```
pub fn noise_2d(seed: impl NoiseSeed, x: f32, y: f32) -> f32 {
    const F2: f64 = 0.366_025_403_784_438_6; // (sqrt 3 - 1) / 2
    const G2: f64 = 0.211_324_865_405_187_1; // (3 - sqrt 3) / 6
    let seed = seed.noise_seed();
    let (x, y) = (f64::from(x), f64::from(y));
    let s = (x + y) * F2;
    let (i, j) = ((x + s).floor(), (y + s).floor());
    let t = (i + j) * G2;
    let (x0, y0) = (x - (i - t), y - (j - t));
    let (i1, j1) = if x0 > y0 { (1.0, 0.0) } else { (0.0, 1.0) };
    let corners = [
        (0.0, 0.0, x0, y0),
        (i1, j1, x0 - i1 + G2, y0 - j1 + G2),
        (1.0, 1.0, x0 - 1.0 + 2.0 * G2, y0 - 1.0 + 2.0 * G2),
    ];
    let sum: f64 = corners
        .iter()
        .map(|&(di, dj, dx, dy)| {
            let t = 0.5 - dx * dx - dy * dy;
            if t < 0.0 {
                return 0.0;
            }
            let cell = [(i + di) as i64, (j + dj) as i64, 0, 0];
            let g = GRAD3[corner(seed, cell, 12)];
            t.powi(4) * (g[0] * dx + g[1] * dy)
        })
        .sum();
    ((70.0 * sum) as f32).clamp(-1.0, 1.0)
}

/// 3D simplex noise at `(x, y, z)`, from -1 to 1 (Remotion's `noise3D`). Use time as `z` to
/// make a 2D field change smoothly.
pub fn noise_3d(seed: impl NoiseSeed, x: f32, y: f32, z: f32) -> f32 {
    const F3: f64 = 1.0 / 3.0;
    const G3: f64 = 1.0 / 6.0;
    let seed = seed.noise_seed();
    let p = [f64::from(x), f64::from(y), f64::from(z)];
    let s = (p[0] + p[1] + p[2]) * F3;
    let cell = p.map(|v| (v + s).floor());
    let t = (cell[0] + cell[1] + cell[2]) * G3;
    let d0 = [
        p[0] - (cell[0] - t),
        p[1] - (cell[1] - t),
        p[2] - (cell[2] - t),
    ];
    // the simplex corners: step along the axes from the largest offset to the smallest
    let mut order = [0, 1, 2];
    order.sort_by(|&a, &b| d0[b].total_cmp(&d0[a]));
    let mut step = [0.0; 3];
    let mut sum = 0.0;
    for n in 0..4 {
        if n > 0 {
            step[order[n - 1]] = 1.0;
        }
        let d: [f64; 3] = std::array::from_fn(|a| d0[a] - step[a] + n as f64 * G3);
        let t = 0.6 - d[0] * d[0] - d[1] * d[1] - d[2] * d[2];
        if t < 0.0 {
            continue;
        }
        let at = [
            (cell[0] + step[0]) as i64,
            (cell[1] + step[1]) as i64,
            (cell[2] + step[2]) as i64,
            0,
        ];
        let g = GRAD3[corner(seed, at, 12)];
        sum += t.powi(4) * (g[0] * d[0] + g[1] * d[1] + g[2] * d[2]);
    }
    ((32.0 * sum) as f32).clamp(-1.0, 1.0)
}

/// 4D simplex noise at `(x, y, z, w)`, from -1 to 1 (Remotion's `noise4D`). Two of the axes
/// can run round a circle to make a 2D field that loops in time.
pub fn noise_4d(seed: impl NoiseSeed, x: f32, y: f32, z: f32, w: f32) -> f32 {
    const F4: f64 = 0.309_016_994_374_947_45; // (sqrt 5 - 1) / 4
    const G4: f64 = 0.138_196_601_125_010_5; // (5 - sqrt 5) / 20
    let seed = seed.noise_seed();
    let p = [f64::from(x), f64::from(y), f64::from(z), f64::from(w)];
    let s = (p[0] + p[1] + p[2] + p[3]) * F4;
    let cell = p.map(|v| (v + s).floor());
    let t = (cell[0] + cell[1] + cell[2] + cell[3]) * G4;
    let d0: [f64; 4] = std::array::from_fn(|a| p[a] - (cell[a] - t));
    let mut order = [0, 1, 2, 3];
    order.sort_by(|&a, &b| d0[b].total_cmp(&d0[a]));
    let mut step = [0.0; 4];
    let mut sum = 0.0;
    for n in 0..5 {
        if n > 0 {
            step[order[n - 1]] = 1.0;
        }
        let d: [f64; 4] = std::array::from_fn(|a| d0[a] - step[a] + n as f64 * G4);
        let t = 0.6 - d.iter().map(|v| v * v).sum::<f64>();
        if t < 0.0 {
            continue;
        }
        let at: [i64; 4] = std::array::from_fn(|a| (cell[a] + step[a]) as i64);
        let g = grad4(corner(seed, at, 32));
        sum += t.powi(4) * (g[0] * d[0] + g[1] * d[1] + g[2] * d[2] + g[3] * d[3]);
    }
    ((27.0 * sum) as f32).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> impl Iterator<Item = (f32, f32)> {
        (0..40).flat_map(|i| (0..40).map(move |j| (i as f32 * 0.137, j as f32 * 0.211)))
    }

    #[test]
    fn noise_is_deterministic_per_seed() {
        for (x, y) in grid().take(50) {
            assert_eq!(noise_2d(7u64, x, y), noise_2d(7u64, x, y));
            assert_eq!(noise_3d("a", x, y, 0.4), noise_3d("a", x, y, 0.4));
            assert_eq!(noise_4d(3, x, y, 0.4, 1.1), noise_4d(3, x, y, 0.4, 1.1));
        }
        assert_eq!("seed".noise_seed(), String::from("seed").noise_seed());
        let differ = grid()
            .filter(|&(x, y)| noise_2d(1, x, y) != noise_2d(2, x, y))
            .count();
        assert!(differ > 1500);
    }

    #[test]
    fn noise_spans_minus_one_to_one_smoothly() {
        let mut lo = 0.0f32;
        let mut hi = 0.0f32;
        for (x, y) in grid() {
            for v in [
                noise_2d(5, x, y),
                noise_3d(5, x, y, 0.7),
                noise_4d(5, x, y, 0.7, 0.3),
            ] {
                assert!((-1.0..=1.0).contains(&v));
                lo = lo.min(v);
                hi = hi.max(v);
            }
            // a small step moves the value a little
            assert!((noise_2d(5, x, y) - noise_2d(5, x + 0.001, y)).abs() < 0.02);
            assert!((noise_3d(5, x, y, 0.7) - noise_3d(5, x, y, 0.701)).abs() < 0.02);
            assert!((noise_4d(5, x, y, 0.7, 0.3) - noise_4d(5, x, y, 0.7, 0.301)).abs() < 0.02);
        }
        assert!(lo < -0.5 && hi > 0.5, "{lo} {hi}");
    }
}
