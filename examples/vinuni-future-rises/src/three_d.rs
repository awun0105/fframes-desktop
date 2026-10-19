//! A tiny flat-shaded 3D renderer that outputs `svgr!` paths: rotate, project with a
//! perspective divide, cull back faces, sort by depth (painter's algorithm) and shade
//! each face by its normal against a directional light. It runs on any backend.

use std::fmt::Write;
use std::ops::{Add, Mul, Sub};

use fframes::Svgr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, k: f32) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
}

impl V3 {
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn normalize(self) -> V3 {
        let len = self.dot(self).sqrt();
        if len > 1e-6 { self * (1.0 / len) } else { self }
    }

    /// Rotation around the vertical axis.
    pub fn rot_y(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c - self.z * s, self.y, self.x * s + self.z * c)
    }

    /// Rotation around the horizontal axis; positive angles tilt the top toward the camera.
    pub fn rot_x(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }

    pub fn rot_z(self, a: f32) -> V3 {
        let (s, c) = a.sin_cos();
        v3(self.x * c - self.y * s, self.x * s + self.y * c, self.z)
    }

    /// A point on the unit sphere, y up, longitude 0 facing the camera.
    pub fn from_lat_lon(lat_deg: f32, lon_deg: f32) -> V3 {
        let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
        v3(lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos())
    }
}

/// A perspective camera at `(0, 0, distance)` looking down -z after the world is rotated
/// by `yaw` then `pitch`; `center` is the screen position of the origin.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub focal: f32,
    pub center: (f32, f32),
}

impl Camera {
    pub fn view(&self, p: V3) -> V3 {
        p.rot_y(self.yaw).rot_x(self.pitch)
    }

    /// Screen position and scale of a point already in view space.
    pub fn project_view(&self, p: V3) -> Option<(f32, f32, f32)> {
        let depth = self.distance - p.z;
        if depth < 1.0 {
            return None;
        }
        let s = self.focal / depth;
        Some((self.center.0 + p.x * s, self.center.1 - p.y * s, s))
    }

    pub fn project(&self, p: V3) -> Option<(f32, f32, f32)> {
        self.project_view(self.view(p))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    pub fn hex(self) -> String {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", c(self.0), c(self.1), c(self.2))
    }

    pub fn scale(self, k: f32) -> Rgb {
        Rgb(self.0 * k, self.1 * k, self.2 * k)
    }

    pub fn mix(self, o: Rgb, t: f32) -> Rgb {
        Rgb(
            self.0 + (o.0 - self.0) * t,
            self.1 + (o.1 - self.1) * t,
            self.2 + (o.2 - self.2) * t,
        )
    }
}

/// A planar polygon, counter-clockwise when seen from its front.
#[derive(Debug, Clone)]
pub struct Face {
    pub points: Vec<V3>,
    pub color: Rgb,
}

/// How faces are lit and outlined.
#[derive(Debug, Clone, Copy)]
pub struct Shading {
    /// Direction toward the light, in view space.
    pub light: V3,
    pub ambient: f32,
    /// Faces fade toward this color with distance.
    pub fog: Rgb,
    pub fog_near: f32,
    pub fog_far: f32,
    pub stroke: Option<(&'static str, f32)>,
}

impl Default for Shading {
    fn default() -> Self {
        Shading {
            light: v3(-0.4, 0.8, 0.6).normalize(),
            ambient: 0.35,
            fog: Rgb(0.02, 0.05, 0.12),
            fog_near: f32::MAX,
            fog_far: f32::MAX,
            stroke: None,
        }
    }
}

/// Projects, culls, sorts and shades `faces`, back to front. `model` places each world
/// point before the camera sees it (for per-object animation).
pub fn render_faces(
    faces: &[Face],
    model: impl Fn(V3) -> V3,
    camera: &Camera,
    shading: &Shading,
) -> Vec<Svgr<'static>> {
    let eye = v3(0.0, 0.0, camera.distance);
    let mut drawn: Vec<(f32, String, String)> = faces
        .iter()
        .filter_map(|face| {
            let pts: Vec<V3> = face.points.iter().map(|&p| camera.view(model(p))).collect();
            if pts.len() < 3 {
                return None;
            }
            // Newell's method: correct for concave outlines like a star, too
            let normal = (0..pts.len())
                .fold(v3(0.0, 0.0, 0.0), |n, i| {
                    n + pts[i].cross(pts[(i + 1) % pts.len()])
                })
                .normalize();
            let centroid =
                pts.iter().fold(v3(0.0, 0.0, 0.0), |a, &p| a + p) * (1.0 / pts.len() as f32);
            if normal.dot(eye - centroid) <= 0.0 {
                return None;
            }
            let mut d = String::with_capacity(pts.len() * 16);
            for (i, &p) in pts.iter().enumerate() {
                let (x, y, _) = camera.project_view(p)?;
                d.push_str(if i == 0 { "M" } else { "L" });
                let _ = write!(d, "{x:.1} {y:.1}");
            }
            d.push('Z');
            let light =
                shading.ambient + (1.0 - shading.ambient) * normal.dot(shading.light).max(0.0);
            let depth = camera.distance - centroid.z;
            let fog =
                ((depth - shading.fog_near) / (shading.fog_far - shading.fog_near)).clamp(0.0, 1.0);
            let color = face.color.scale(light).mix(shading.fog, fog);
            Some((depth, d, color.hex()))
        })
        .collect();
    drawn.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (stroke, stroke_width) = shading.stroke.unwrap_or(("none", 0.0));
    drawn
        .into_iter()
        .map(|(_, d, fill)| {
            fframes::svgr!(<path d={d} fill={fill} stroke={stroke} stroke-width={stroke_width} stroke-linejoin="round" />)
        })
        .collect()
}

/// An axis-aligned box from `min` with `size`; the bottom face is left out.
pub fn cuboid(min: V3, size: V3, color: Rgb, top: Rgb) -> Vec<Face> {
    let (x0, y0, z0) = (min.x, min.y, min.z);
    let (x1, y1, z1) = (min.x + size.x, min.y + size.y, min.z + size.z);
    let quad = |a, b, c, d, color| Face {
        points: vec![a, b, c, d],
        color,
    };
    vec![
        quad(
            v3(x0, y0, z1),
            v3(x1, y0, z1),
            v3(x1, y1, z1),
            v3(x0, y1, z1),
            color,
        ), // front +z
        quad(
            v3(x1, y0, z0),
            v3(x0, y0, z0),
            v3(x0, y1, z0),
            v3(x1, y1, z0),
            color,
        ), // back -z
        quad(
            v3(x1, y0, z1),
            v3(x1, y0, z0),
            v3(x1, y1, z0),
            v3(x1, y1, z1),
            color,
        ), // right +x
        quad(
            v3(x0, y0, z0),
            v3(x0, y0, z1),
            v3(x0, y1, z1),
            v3(x0, y1, z0),
            color,
        ), // left -x
        quad(
            v3(x0, y1, z1),
            v3(x1, y1, z1),
            v3(x1, y1, z0),
            v3(x0, y1, z0),
            top,
        ), // top +y
    ]
}

/// A 2D polygon (counter-clockwise, in the xy plane) extruded along z by `depth`.
pub fn extrude(outline: &[(f32, f32)], depth: f32, front: Rgb, side: Rgb) -> Vec<Face> {
    let h = depth / 2.0;
    let mut faces = vec![
        Face {
            points: outline.iter().map(|&(x, y)| v3(x, y, h)).collect(),
            color: front,
        },
        Face {
            points: outline.iter().rev().map(|&(x, y)| v3(x, y, -h)).collect(),
            color: front,
        },
    ];
    for i in 0..outline.len() {
        let (ax, ay) = outline[i];
        let (bx, by) = outline[(i + 1) % outline.len()];
        faces.push(Face {
            points: vec![v3(ax, ay, h), v3(ax, ay, -h), v3(bx, by, -h), v3(bx, by, h)],
            color: side,
        });
    }
    faces
}

/// A regular icosahedron of circumradius `r`, faces alternating between two colors.
pub fn icosahedron(r: f32, a: Rgb, b: Rgb) -> Vec<Face> {
    const TRIS: [(usize, usize, usize); 20] = [
        (0, 11, 5),
        (0, 5, 1),
        (0, 1, 7),
        (0, 7, 10),
        (0, 10, 11),
        (1, 5, 9),
        (5, 11, 4),
        (11, 10, 2),
        (10, 7, 6),
        (7, 1, 8),
        (3, 9, 4),
        (3, 4, 2),
        (3, 2, 6),
        (3, 6, 8),
        (3, 8, 9),
        (4, 9, 5),
        (2, 4, 11),
        (6, 2, 10),
        (8, 6, 7),
        (9, 8, 1),
    ];
    let t = f32::midpoint(1.0, 5f32.sqrt());
    let verts: Vec<V3> = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| v3(x, y, z).normalize() * r)
    .collect();
    TRIS.iter()
        .enumerate()
        .map(|(i, &(p, q, s))| {
            // the table winds clockwise seen from outside; flip it to face out
            Face {
                points: vec![verts[p], verts[s], verts[q]],
                color: if i % 2 == 0 { a } else { b },
            }
        })
        .collect()
}

/// A polyline through 3D points as one path, split where points fall behind the camera
/// or fail `visible`.
pub fn polyline(points: &[V3], camera: &Camera, visible: impl Fn(V3) -> bool) -> String {
    let mut d = String::new();
    let mut pen_down = false;
    for &p in points {
        let v = camera.view(p);
        match camera.project_view(v).filter(|_| visible(v)) {
            Some((x, y, _)) => {
                d.push_str(if pen_down { "L" } else { "M" });
                let _ = write!(d, "{x:.1} {y:.1}");
                pen_down = true;
            }
            None => pen_down = false,
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_bring_a_point_to_the_front() {
        let p = V3::from_lat_lon(21.0, 105.85);
        let front = p.rot_y(105.85f32.to_radians()).rot_x(21f32.to_radians());
        assert!((front.z - 1.0).abs() < 1e-4, "{front:?}");
    }

    #[test]
    fn closed_solids_face_outward() {
        let camera = Camera {
            yaw: 0.3,
            pitch: 0.4,
            distance: 10.0,
            focal: 800.0,
            center: (0.0, 0.0),
        };
        let shading = Shading::default();
        // a convex solid seen from outside shows between one and half of its faces
        let ico = render_faces(
            &icosahedron(1.0, Rgb(1.0, 0.0, 0.0), Rgb(0.0, 0.0, 1.0)),
            |p| p,
            &camera,
            &shading,
        );
        assert!((5..=12).contains(&ico.len()), "{}", ico.len());
        let prism = render_faces(
            &extrude(
                &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
                0.5,
                Rgb(1.0, 1.0, 1.0),
                Rgb(0.5, 0.5, 0.5),
            ),
            |p| p,
            &camera,
            &shading,
        );
        assert!((2..=3).contains(&prism.len()), "{}", prism.len());
        // a concave cap seen head-on stays visible
        let head_on = Camera {
            yaw: 0.0,
            pitch: 0.0,
            ..camera
        };
        // starts at the reflex vertex, where the first three points wind the wrong way
        let arrow = [(0.0, 1.0), (0.4, 0.5), (0.0, 0.0), (1.0, 0.5)];
        let caps = render_faces(
            &extrude(&arrow, 0.5, Rgb(1.0, 1.0, 1.0), Rgb(0.5, 0.5, 0.5)),
            |p| p,
            &head_on,
            &shading,
        );
        assert!(!caps.is_empty());
    }
}
