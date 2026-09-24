use super::impl_rotating_scene;
use crate::error::{Error, Result};
use crate::renderer::{AsciiBuffer, Mat3, RenderMode, Renderer, Vec3};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Faces meeting at a sharper angle than this keep a hard edge (60 degrees)
const SMOOTHING_ANGLE: f32 = std::f32::consts::FRAC_PI_3;

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    /// Face normal
    pub normal: Vec3,
    /// Per-corner normals for smooth shading (computed by `Mesh::new`)
    pub vertex_normals: [Vec3; 3],
}

impl Triangle {
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        // Calculate face normal from vertices
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = Vec3::new(
            edge1.y * edge2.z - edge1.z * edge2.y,
            edge1.z * edge2.x - edge1.x * edge2.z,
            edge1.x * edge2.y - edge1.y * edge2.x,
        ).normalize();

        Self {
            vertices: [v0, v1, v2],
            normal,
            vertex_normals: [normal; 3],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
    pub scale: f32,
    pub center: Vec3,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.5, 0.7, 0.3),
            scale: 1.0,
            center: Vec3::default(),
        }
    }
}

impl Mesh {
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let mut mesh = Self {
            triangles,
            ..Default::default()
        };
        mesh.recenter();
        mesh.normalize_scale();
        mesh.smooth_normals(SMOOTHING_ANGLE);
        mesh
    }

    pub fn with_rotation_speed(mut self, speed: Vec3) -> Self {
        self.rotation_speed = speed;
        self
    }

    /// Load mesh from OBJ file
    pub fn from_obj<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (models, _materials) = tobj::load_obj(
            path.as_ref(),
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        ).map_err(|e| Error::InvalidData(format!("OBJ: {e}")))?;

        let mut triangles = Vec::new();

        for model in models {
            let mesh = &model.mesh;
            let positions = &mesh.positions;
            let indices = &mesh.indices;
            let normals = &mesh.normals;

            // Process triangles
            for chunk in indices.chunks(3) {
                if chunk.len() == 3 {
                    let i0 = chunk[0] as usize;
                    let i1 = chunk[1] as usize;
                    let i2 = chunk[2] as usize;

                    let v0 = Vec3::new(
                        positions[i0 * 3],
                        positions[i0 * 3 + 1],
                        positions[i0 * 3 + 2],
                    );
                    let v1 = Vec3::new(
                        positions[i1 * 3],
                        positions[i1 * 3 + 1],
                        positions[i1 * 3 + 2],
                    );
                    let v2 = Vec3::new(
                        positions[i2 * 3],
                        positions[i2 * 3 + 1],
                        positions[i2 * 3 + 2],
                    );

                    let mut tri = Triangle::new(v0, v1, v2);

                    // Use provided normals if available (average vertex normals for face)
                    let normal_at = |i: usize| {
                        normals.get(i * 3..i * 3 + 3).map(|n| Vec3::new(n[0], n[1], n[2]))
                    };
                    if let (Some(n0), Some(n1), Some(n2)) = (normal_at(i0), normal_at(i1), normal_at(i2)) {
                        let avg = n0 + n1 + n2;
                        if avg.dot(avg) > 0.0 {
                            tri.normal = avg.normalize();
                        }
                    }

                    triangles.push(tri);
                }
            }
        }

        if triangles.is_empty() {
            return Err(Error::InvalidData("no triangles found in OBJ file".into()));
        }

        Ok(Self::new(triangles))
    }

    /// Calculate center of mass and translate to origin
    fn recenter(&mut self) {
        let mut sum = Vec3::default();
        let mut count = 0;

        for tri in &self.triangles {
            for v in &tri.vertices {
                sum = sum + *v;
                count += 1;
            }
        }

        if count > 0 {
            self.center = sum * (1.0 / count as f32);

            // Translate all vertices to center at origin
            for tri in &mut self.triangles {
                for v in &mut tri.vertices {
                    *v = *v - self.center;
                }
            }
        }
    }

    /// Scale mesh to fit in a unit sphere
    fn normalize_scale(&mut self) {
        let mut max_dist: f32 = 0.0;

        for tri in &self.triangles {
            for v in &tri.vertices {
                let dist = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
                max_dist = max_dist.max(dist);
            }
        }

        if max_dist > 0.0 {
            self.scale = 2.0 / max_dist; // Scale to fit in radius 2
            for tri in &mut self.triangles {
                for v in &mut tri.vertices {
                    *v = *v * self.scale;
                }
            }
        }
    }

    /// Recompute per-corner normals by averaging the face normals of all
    /// triangles sharing that corner, skipping faces that meet at more than
    /// `max_angle` radians so hard edges stay sharp.
    pub fn smooth_normals(&mut self, max_angle: f32) {
        use std::collections::HashMap;

        // Weld corners by (quantized) position
        let key = |v: Vec3| {
            let q = |c: f32| (c * 1e4).round() as i32;
            (q(v.x), q(v.y), q(v.z))
        };
        let mut faces_at: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (i, tri) in self.triangles.iter().enumerate() {
            for v in tri.vertices {
                faces_at.entry(key(v)).or_default().push(i);
            }
        }

        let min_cos = max_angle.cos();
        let normals: Vec<[Vec3; 3]> = self
            .triangles
            .iter()
            .map(|tri| {
                tri.vertices.map(|v| {
                    let sum = faces_at[&key(v)]
                        .iter()
                        .map(|&j| self.triangles[j].normal)
                        .filter(|n| n.dot(tri.normal) >= min_cos)
                        .fold(Vec3::ZERO, |acc, n| acc + n);
                    if sum.dot(sum) > 0.0 { sum.normalize() } else { tri.normal }
                })
            })
            .collect();

        for (tri, n) in self.triangles.iter_mut().zip(normals) {
            tri.vertex_normals = n;
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    pub fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let rot = Mat3::from_rotation(self.rotation);
        for tri in &self.triangles {
            let [a, b, c] = tri.vertices.map(|v| rot.transform(v));
            match renderer.mode {
                RenderMode::Solid => {
                    renderer.draw_triangle(buffer, [a, b, c], tri.vertex_normals.map(|n| rot.transform(n)))
                }
                RenderMode::Wireframe => {
                    let n = rot.transform(tri.normal);
                    renderer.draw_line(buffer, a, b, n, n);
                    renderer.draw_line(buffer, b, c, n, n);
                    renderer.draw_line(buffer, c, a, n, n);
                }
            }
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.rotation_speed = Vec3::new(0.5, 0.7, 0.3) * multiplier;
    }

    /// Load mesh from STL file (ASCII or binary)
    pub fn from_stl<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);

        // Check if ASCII or binary by reading first bytes
        let mut header = [0u8; 80];
        reader.read_exact(&mut header)?;

        // Many binary exporters also start the header with "solid", so trust
        // the binary layout whenever the file size matches it exactly.
        let mut count_bytes = [0u8; 4];
        let binary_size_matches = reader.read_exact(&mut count_bytes).is_ok()
            && std::fs::metadata(path.as_ref()).is_ok_and(|m| {
                m.len() == 84 + 50 * u64::from(u32::from_le_bytes(count_bytes))
            });
        let header_str = String::from_utf8_lossy(&header);
        let is_ascii = !binary_size_matches && header_str.trim_start().starts_with("solid");

        // Reopen file for proper parsing
        let file = File::open(path.as_ref())?;

        let triangles = if is_ascii {
            Self::parse_stl_ascii(file)?
        } else {
            Self::parse_stl_binary(file)?
        };

        if triangles.is_empty() {
            return Err(Error::InvalidData("no triangles found in STL file".into()));
        }

        Ok(Self::new(triangles))
    }

    fn parse_stl_ascii(file: File) -> Result<Vec<Triangle>> {
        let reader = BufReader::new(file);
        let mut triangles = Vec::new();
        let mut current_normal = Vec3::default();
        let mut vertices: Vec<Vec3> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "facet" if parts.len() >= 5 && parts[1] == "normal" => {
                    let nx: f32 = parts[2].parse().unwrap_or(0.0);
                    let ny: f32 = parts[3].parse().unwrap_or(0.0);
                    let nz: f32 = parts[4].parse().unwrap_or(0.0);
                    current_normal = Vec3::new(nx, ny, nz).normalize();
                }
                "vertex" if parts.len() >= 4 => {
                    let x: f32 = parts[1].parse().unwrap_or(0.0);
                    let y: f32 = parts[2].parse().unwrap_or(0.0);
                    let z: f32 = parts[3].parse().unwrap_or(0.0);
                    vertices.push(Vec3::new(x, y, z));

                    if vertices.len() == 3 {
                        let mut tri = Triangle::new(vertices[0], vertices[1], vertices[2]);
                        if current_normal.x != 0.0 || current_normal.y != 0.0 || current_normal.z != 0.0 {
                            tri.normal = current_normal;
                        }
                        triangles.push(tri);
                        vertices.clear();
                    }
                }
                _ => {}
            }
        }

        Ok(triangles)
    }

    fn parse_stl_binary(file: File) -> Result<Vec<Triangle>> {
        let mut reader = BufReader::new(file);

        // Skip 80-byte header
        let mut header = [0u8; 80];
        reader.read_exact(&mut header)?;

        // Read triangle count (4 bytes, little endian)
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let triangle_count = u32::from_le_bytes(count_bytes) as usize;

        let mut triangles = Vec::with_capacity(triangle_count);

        for _ in 0..triangle_count {
            // Each triangle: normal (3 floats) + 3 vertices (9 floats) + attribute (2 bytes) = 50 bytes
            let mut data = [0u8; 50];
            if reader.read_exact(&mut data).is_err() {
                break;
            }

            let read_f32 = |offset: usize| -> f32 {
                f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
            };

            let normal = Vec3::new(read_f32(0), read_f32(4), read_f32(8)).normalize();
            let v0 = Vec3::new(read_f32(12), read_f32(16), read_f32(20));
            let v1 = Vec3::new(read_f32(24), read_f32(28), read_f32(32));
            let v2 = Vec3::new(read_f32(36), read_f32(40), read_f32(44));

            let mut tri = Triangle::new(v0, v1, v2);
            if normal.x != 0.0 || normal.y != 0.0 || normal.z != 0.0 {
                tri.normal = normal;
            }
            triangles.push(tri);
        }

        Ok(triangles)
    }

    /// Load mesh from file, auto-detecting format by extension
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "obj" => Self::from_obj(path),
            "stl" => Self::from_stl(path),
            _ => Err(Error::UnsupportedFormat(ext)),
        }
    }
}

impl_rotating_scene!(Mesh);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_stl_with_solid_header() {
        // Binary STL whose header starts with "solid", as many exporters write
        let mut data = Vec::new();
        let mut header = [b' '; 80];
        header[..11].copy_from_slice(b"solid model");
        data.extend_from_slice(&header);
        data.extend_from_slice(&1u32.to_le_bytes());
        let floats: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        for f in floats {
            data.extend_from_slice(&f.to_le_bytes());
        }
        data.extend_from_slice(&[0u8; 2]);

        let path = std::env::temp_dir().join(format!("zoa_solid_header_{}.stl", std::process::id()));
        std::fs::write(&path, &data).unwrap();
        let mesh = Mesh::from_stl(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(mesh.expect("binary STL should parse").triangle_count(), 1);
    }

    #[test]
    fn test_smooth_normals_keep_hard_edges() {
        // Two coplanar triangles share smoothed normals; a 90-degree fold keeps
        // each face's own normal
        let flat = Mesh::new(vec![
            Triangle::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
            Triangle::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
        ]);
        assert_eq!(flat.triangles[0].vertex_normals[0], flat.triangles[0].normal);

        let fold = Mesh::new(vec![
            Triangle::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
            Triangle::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0)),
        ]);
        for tri in &fold.triangles {
            assert!(tri.vertex_normals.iter().all(|&n| (n - tri.normal).length() < 1e-5));
        }

        // A gentle bend is smoothed across the shared edge
        let bend = Mesh::new(vec![
            Triangle::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            Triangle::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.3), Vec3::new(0.0, 1.0, 0.0)),
        ]);
        let shared = bend.triangles[0].vertex_normals[1];
        assert!((shared - bend.triangles[0].normal).length() > 1e-3);
        assert!((shared - bend.triangles[1].vertex_normals[0]).length() < 1e-5);
    }

    #[test]
    fn test_sample_meshes_load() {
        for name in ["samples/bunny.stl", "samples/tetrahedron.stl", "samples/pyramid.obj", "samples/icosahedron.obj"] {
            let path = Path::new(name);
            if path.exists() {
                let mesh = Mesh::from_file(path).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert!(mesh.triangle_count() > 0);
            }
        }
    }
}
