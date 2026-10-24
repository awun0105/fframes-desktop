use fframes::{
    Color,
    three_d::{Geometry, Material, Scene, Vec3},
};

use crate::{
    theme::Theme,
    ui::{Themed, Ui},
};

use super::{Dot3D, Enter3D, Object3D, Point3, place, rgb, world};

/// The most points [`ConvexHull3D`] takes; it compares every triple of them.
pub const MAX_HULL_POINTS: usize = 64;

/// Manim's `Polyhedron`: `faces` are lists of indices into `vertices`, each a flat polygon
/// filled at opacity 0.5, and every vertex is a [`Dot3D`] (`show_vertices`). Edges are not
/// drawn, as in Manim. `center` moves it and `enter` animates the whole solid.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyhedron {
    pub vertices: Vec<Point3>,
    pub faces: Vec<Vec<usize>>,
    pub center: Point3,
    pub fill_color: Color,
    pub opacity: f32,
    pub show_vertices: bool,
    pub vertex_color: Color,
    pub enter: Option<Enter3D>,
}

impl Themed for Polyhedron {
    /// No vertices until they are set.
    fn themed(t: &Theme) -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            center: (0.0, 0.0, 0.0),
            fill_color: t.colors.accent,
            opacity: 0.5,
            show_vertices: true,
            vertex_color: t.colors.title,
            enter: None,
        }
    }
}

impl Default for Polyhedron {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl Polyhedron {
    /// The faces as triangles of world points around the center, fanned from each face's
    /// first vertex. Indices outside `vertices` drop their face.
    fn triangles(&self) -> Vec<Vec3> {
        let mut out = Vec::new();
        for face in &self.faces {
            let points: Option<Vec<Vec3>> = face
                .iter()
                .map(|&i| self.vertices.get(i).map(|&p| world(p)))
                .collect();
            let Some(points) = points else { continue };
            for k in 1..points.len().saturating_sub(1) {
                out.extend([points[0], points[k], points[k + 1]]);
            }
        }
        out
    }
}

impl Object3D for Polyhedron {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        let triangles = self.triangles();
        if !triangles.is_empty() {
            let material = Material::standard(rgb(self.fill_color))
                .opacity(self.opacity)
                .double_sided()
                .flat();
            let geometry = Geometry::from_triangles(&triangles);
            place(
                ui,
                scene,
                geometry,
                material,
                self.center,
                self.enter.as_ref(),
            );
        }
        if self.show_vertices {
            let c = self.center;
            for &(x, y, z) in &self.vertices {
                Dot3D {
                    point: (c.0 + x, c.1 + y, c.2 + z),
                    color: self.vertex_color,
                    enter: self.enter,
                    ..Dot3D::default()
                }
                .add_to(ui, scene);
            }
        }
    }
}

macro_rules! platonic {
    ($(#[$doc:meta])* $ty:ident, $build:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $ty {
            pub edge_length: f32,
            pub center: Point3,
            pub fill_color: Color,
            pub opacity: f32,
            pub show_vertices: bool,
            pub vertex_color: Color,
            pub enter: Option<Enter3D>,
        }

        impl Themed for $ty {
            fn themed(t: &Theme) -> Self {
                let p = Polyhedron::themed(t);
                Self {
                    edge_length: 1.0,
                    center: p.center,
                    fill_color: p.fill_color,
                    opacity: p.opacity,
                    show_vertices: p.show_vertices,
                    vertex_color: p.vertex_color,
                    enter: None,
                }
            }
        }

        impl Default for $ty {
            fn default() -> Self {
                Self::themed(&Theme::default())
            }
        }

        impl $ty {
            /// The [`Polyhedron`] with these vertices and faces.
            pub fn polyhedron(&self) -> Polyhedron {
                let (vertices, faces) = $build(self.edge_length);
                Polyhedron {
                    vertices,
                    faces: faces.iter().map(|f| f.to_vec()).collect(),
                    center: self.center,
                    fill_color: self.fill_color,
                    opacity: self.opacity,
                    show_vertices: self.show_vertices,
                    vertex_color: self.vertex_color,
                    enter: self.enter,
                }
            }
        }

        impl Object3D for $ty {
            fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
                self.polyhedron().add_to(ui, scene);
            }
        }
    };
}

fn tetrahedron(e: f32) -> (Vec<Point3>, Vec<[usize; 3]>) {
    let u = e * 2f32.sqrt() / 4.0;
    let vertices = vec![(u, u, u), (u, -u, -u), (-u, u, -u), (-u, -u, u)];
    (vertices, vec![[0, 1, 2], [3, 0, 2], [0, 1, 3], [3, 1, 2]])
}

fn octahedron(e: f32) -> (Vec<Point3>, Vec<[usize; 3]>) {
    let u = e * 2f32.sqrt() / 2.0;
    let vertices = vec![
        (u, 0.0, 0.0),
        (-u, 0.0, 0.0),
        (0.0, u, 0.0),
        (0.0, -u, 0.0),
        (0.0, 0.0, u),
        (0.0, 0.0, -u),
    ];
    let faces = vec![
        [2, 4, 1],
        [0, 4, 2],
        [4, 3, 0],
        [1, 3, 4],
        [3, 5, 0],
        [1, 5, 3],
        [2, 5, 1],
        [0, 5, 2],
    ];
    (vertices, faces)
}

fn icosahedron(e: f32) -> (Vec<Point3>, Vec<[usize; 3]>) {
    let a = e * (1.0 + 5f32.sqrt()) / 4.0;
    let b = e / 2.0;
    let vertices = vec![
        (0.0, b, a),
        (0.0, -b, a),
        (0.0, b, -a),
        (0.0, -b, -a),
        (b, a, 0.0),
        (b, -a, 0.0),
        (-b, a, 0.0),
        (-b, -a, 0.0),
        (a, 0.0, b),
        (a, 0.0, -b),
        (-a, 0.0, b),
        (-a, 0.0, -b),
    ];
    let faces = vec![
        [1, 8, 0],
        [1, 5, 7],
        [8, 5, 1],
        [7, 3, 5],
        [5, 9, 3],
        [8, 9, 5],
        [3, 2, 9],
        [9, 4, 2],
        [8, 4, 9],
        [0, 4, 8],
        [6, 4, 0],
        [6, 2, 4],
        [11, 2, 6],
        [3, 11, 2],
        [0, 6, 10],
        [10, 1, 0],
        [10, 7, 1],
        [11, 7, 3],
        [10, 11, 7],
        [10, 11, 6],
    ];
    (vertices, faces)
}

fn dodecahedron(e: f32) -> (Vec<Point3>, Vec<[usize; 5]>) {
    let a = e * (1.0 + 5f32.sqrt()) / 4.0;
    let b = e * (3.0 + 5f32.sqrt()) / 4.0;
    let c = e / 2.0;
    let vertices = vec![
        (a, a, a),
        (a, a, -a),
        (a, -a, a),
        (a, -a, -a),
        (-a, a, a),
        (-a, a, -a),
        (-a, -a, a),
        (-a, -a, -a),
        (0.0, c, b),
        (0.0, c, -b),
        (0.0, -c, -b),
        (0.0, -c, b),
        (c, b, 0.0),
        (-c, b, 0.0),
        (c, -b, 0.0),
        (-c, -b, 0.0),
        (b, 0.0, c),
        (-b, 0.0, c),
        (b, 0.0, -c),
        (-b, 0.0, -c),
    ];
    let faces = vec![
        [18, 16, 0, 12, 1],
        [3, 18, 16, 2, 14],
        [3, 10, 9, 1, 18],
        [1, 9, 5, 13, 12],
        [0, 8, 4, 13, 12],
        [2, 16, 0, 8, 11],
        [4, 17, 6, 11, 8],
        [17, 19, 5, 13, 4],
        [19, 7, 15, 6, 17],
        [6, 15, 14, 2, 11],
        [19, 5, 9, 10, 7],
        [7, 10, 3, 14, 15],
    ];
    (vertices, faces)
}

platonic!(
    /// Manim's `Tetrahedron`: 4 triangles, edges `edge_length` (1) long.
    Tetrahedron,
    tetrahedron
);
platonic!(
    /// Manim's `Octahedron`: 8 triangles, edges `edge_length` (1) long.
    Octahedron,
    octahedron
);
platonic!(
    /// Manim's `Icosahedron`: 20 triangles, edges `edge_length` (1) long.
    Icosahedron,
    icosahedron
);
platonic!(
    /// Manim's `Dodecahedron`: 12 pentagons, edges `edge_length` (1) long.
    Dodecahedron,
    dodecahedron
);

/// The faces of the convex hull of `points`, as indices into it: each face is a flat
/// polygon ordered counterclockwise seen from outside, coplanar points merged into one face.
/// Points closer than 1e-5 to a face count as on it. Fewer than four points that are not
/// all in one plane give no faces, and only the first [`MAX_HULL_POINTS`] points are used.
pub fn convex_hull_faces(points: &[Point3]) -> Vec<Vec<usize>> {
    const EPS: f32 = 1e-5;
    let p: Vec<Vec3> = points
        .iter()
        .take(MAX_HULL_POINTS)
        .map(|&(x, y, z)| Vec3::new(x, y, z))
        .collect();
    let n = p.len();
    let scale = p
        .iter()
        .fold(0.0f32, |m, v| m.max(v.abs().max_element()))
        .max(1.0);
    let eps = EPS * scale;
    let mut planes: Vec<(Vec3, f32)> = Vec::new();
    let mut faces = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                let normal = (p[j] - p[i]).cross(p[k] - p[i]);
                let len = normal.length();
                if len <= eps * eps {
                    continue;
                }
                let normal = normal / len;
                let d = normal.dot(p[i]);
                let (mut above, mut below) = (false, false);
                for q in &p {
                    let s = normal.dot(*q) - d;
                    above |= s > eps;
                    below |= s < -eps;
                }
                if above && below {
                    continue;
                }
                // outward: every point is on the inner side
                let (normal, d) = if above { (-normal, -d) } else { (normal, d) };
                let known = planes
                    .iter()
                    .any(|(m, e)| m.dot(normal) > 1.0 - 1e-4 && (e - d).abs() <= eps);
                if known {
                    continue;
                }
                planes.push((normal, d));
                let on: Vec<usize> = (0..n)
                    .filter(|&q| (normal.dot(p[q]) - d).abs() <= eps)
                    .collect();
                faces.push(order_around(&p, on, normal));
            }
        }
    }
    faces
}

/// `face` sorted counterclockwise around its centroid seen from the side `normal` points
/// to, with points on an edge between two corners dropped.
fn order_around(p: &[Vec3], face: Vec<usize>, normal: Vec3) -> Vec<usize> {
    let centroid = face.iter().map(|&i| p[i]).sum::<Vec3>() / face.len() as f32;
    let x = (p[face[0]] - centroid).normalize_or(normal.any_orthonormal_vector());
    let y = normal.cross(x);
    let mut sorted: Vec<(f32, usize)> = face
        .into_iter()
        .map(|i| {
            let d = p[i] - centroid;
            (d.dot(y).atan2(d.dot(x)), i)
        })
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let ring: Vec<usize> = sorted.into_iter().map(|(_, i)| i).collect();
    let m = ring.len();
    (0..m)
        .filter(|&k| {
            let (a, b, c) = (p[ring[(k + m - 1) % m]], p[ring[k]], p[ring[(k + 1) % m]]);
            (b - a).cross(c - b).length() > 1e-6 * (b - a).length().max(1e-6)
        })
        .map(|k| ring[k])
        .collect()
}

/// Manim's `ConvexHull3D`: the convex hull of `points` as a [`Polyhedron`], points that are
/// not corners shown as dots too.
#[derive(Debug, Clone, PartialEq)]
pub struct ConvexHull3D {
    pub points: Vec<Point3>,
    pub center: Point3,
    pub fill_color: Color,
    pub opacity: f32,
    pub show_vertices: bool,
    pub vertex_color: Color,
    pub enter: Option<Enter3D>,
}

impl Themed for ConvexHull3D {
    fn themed(t: &Theme) -> Self {
        let p = Polyhedron::themed(t);
        Self {
            points: Vec::new(),
            center: p.center,
            fill_color: p.fill_color,
            opacity: p.opacity,
            show_vertices: p.show_vertices,
            vertex_color: p.vertex_color,
            enter: None,
        }
    }
}

impl Default for ConvexHull3D {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ConvexHull3D {
    /// The hull as a [`Polyhedron`].
    pub fn polyhedron(&self) -> Polyhedron {
        Polyhedron {
            vertices: self.points.iter().take(MAX_HULL_POINTS).copied().collect(),
            faces: convex_hull_faces(&self.points),
            center: self.center,
            fill_color: self.fill_color,
            opacity: self.opacity,
            show_vertices: self.show_vertices,
            vertex_color: self.vertex_color,
            enter: self.enter,
        }
    }
}

impl Object3D for ConvexHull3D {
    fn add_to(&self, ui: &Ui<'_, '_, '_>, scene: &mut Scene) {
        self.polyhedron().add_to(ui, scene);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(p: Polyhedron) -> (usize, usize) {
        (p.vertices.len(), p.faces.len())
    }

    fn edge(p: &Polyhedron) -> f32 {
        let f = &p.faces[0];
        let (a, b) = (world(p.vertices[f[0]]), world(p.vertices[f[1]]));
        a.distance(b)
    }

    #[test]
    fn the_platonic_solids_match_manim() {
        assert_eq!(counts(Tetrahedron::default().polyhedron()), (4, 4));
        assert_eq!(counts(Octahedron::default().polyhedron()), (6, 8));
        assert_eq!(counts(Icosahedron::default().polyhedron()), (12, 20));
        assert_eq!(counts(Dodecahedron::default().polyhedron()), (20, 12));
        for p in [
            Tetrahedron::default().polyhedron(),
            Octahedron::default().polyhedron(),
            Icosahedron::default().polyhedron(),
            Dodecahedron::default().polyhedron(),
        ] {
            assert!((edge(&p) - 1.0).abs() < 1e-5, "{}", edge(&p));
        }
    }

    #[test]
    fn the_hull_of_a_cube_has_six_squares() {
        let mut points = Vec::new();
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    points.push((x, y, z));
                }
            }
        }
        // the middle and a point on an edge are not corners
        points.push((0.0, 0.0, 0.0));
        points.push((1.0, 1.0, 0.0));
        let faces = convex_hull_faces(&points);
        assert_eq!(faces.len(), 6);
        assert!(faces.iter().all(|f| f.len() == 4), "{faces:?}");
        assert!(faces.iter().all(|f| !f.contains(&8) && !f.contains(&9)));
    }

    #[test]
    fn the_hull_of_a_tetrahedron_faces_outward() {
        let (vertices, _) = tetrahedron(1.0);
        let faces = convex_hull_faces(&vertices);
        assert_eq!(faces.len(), 4);
        for f in faces {
            let p: Vec<Vec3> = f.iter().map(|&i| Vec3::from(vertices[i])).collect();
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            assert!(normal.dot(p[0]) > 0.0);
        }
    }
}
