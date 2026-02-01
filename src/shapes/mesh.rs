use crate::renderer::{AsciiBuffer, Renderer, Vec3};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
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
        }
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
    pub scale: f32,
    pub center: Vec3,
    density: usize, // points per triangle edge
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.5, 0.7, 0.3),
            scale: 1.0,
            center: Vec3::default(),
            density: 8,
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
        mesh
    }

    pub fn with_density(mut self, density: usize) -> Self {
        self.density = density;
        self
    }

    pub fn with_rotation_speed(mut self, speed: Vec3) -> Self {
        self.rotation_speed = speed;
        self
    }

    /// Load mesh from OBJ file
    pub fn from_obj<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let (models, _materials) = tobj::load_obj(
            path.as_ref(),
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        ).map_err(|e| format!("Failed to load OBJ: {}", e))?;

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
                    if !normals.is_empty() && i0 * 3 + 2 < normals.len() {
                        let n0 = Vec3::new(
                            normals[i0 * 3],
                            normals[i0 * 3 + 1],
                            normals[i0 * 3 + 2],
                        );
                        tri.normal = n0.normalize();
                    }

                    triangles.push(tri);
                }
            }
        }

        if triangles.is_empty() {
            return Err("No triangles found in OBJ file".to_string());
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

    pub fn update(&mut self, dt: f32) {
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    pub fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        for tri in &self.triangles {
            self.render_triangle(renderer, buffer, tri);
        }
    }

    fn render_triangle(&self, renderer: &Renderer, buffer: &mut AsciiBuffer, tri: &Triangle) {
        // Sample points across the triangle surface using barycentric coordinates
        let steps = self.density;

        for i in 0..=steps {
            for j in 0..=(steps - i) {
                let u = i as f32 / steps as f32;
                let v = j as f32 / steps as f32;
                let w = 1.0 - u - v;

                if w >= 0.0 {
                    // Interpolate position using barycentric coordinates
                    let position = tri.vertices[0] * w + tri.vertices[1] * u + tri.vertices[2] * v;

                    // Apply rotations (Y first for world-axis manual control)
                    let rotated_pos = position
                        .rotate_y(self.rotation.y)
                        .rotate_x(self.rotation.x)
                        .rotate_z(self.rotation.z);

                    let rotated_normal = tri.normal
                        .rotate_y(self.rotation.y)
                        .rotate_x(self.rotation.x)
                        .rotate_z(self.rotation.z)
                        .normalize();

                    renderer.render_point(buffer, rotated_pos, rotated_normal);
                }
            }
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn set_density(&mut self, density: usize) {
        self.density = density.max(1);
    }

    pub fn get_density(&self) -> usize {
        self.density
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.rotation_speed = Vec3::new(0.5, 0.7, 0.3) * multiplier;
    }

    /// Load mesh from STL file (ASCII or binary)
    pub fn from_stl<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("Failed to open STL: {}", e))?;
        let mut reader = BufReader::new(file);

        // Check if ASCII or binary by reading first bytes
        let mut header = [0u8; 80];
        reader.read_exact(&mut header)
            .map_err(|e| format!("Failed to read STL header: {}", e))?;

        // Check if it starts with "solid" (ASCII) but also check it's not binary
        let header_str = String::from_utf8_lossy(&header);
        let is_ascii = header_str.trim_start().starts_with("solid");

        // Reopen file for proper parsing
        let file = File::open(path.as_ref())
            .map_err(|e| format!("Failed to reopen STL: {}", e))?;

        let triangles = if is_ascii {
            Self::parse_stl_ascii(file)?
        } else {
            Self::parse_stl_binary(file)?
        };

        if triangles.is_empty() {
            return Err("No triangles found in STL file".to_string());
        }

        Ok(Self::new(triangles))
    }

    fn parse_stl_ascii(file: File) -> Result<Vec<Triangle>, String> {
        let reader = BufReader::new(file);
        let mut triangles = Vec::new();
        let mut current_normal = Vec3::default();
        let mut vertices: Vec<Vec3> = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Read error: {}", e))?;
            let parts: Vec<&str> = line.trim().split_whitespace().collect();

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

    fn parse_stl_binary(file: File) -> Result<Vec<Triangle>, String> {
        let mut reader = BufReader::new(file);

        // Skip 80-byte header
        let mut header = [0u8; 80];
        reader.read_exact(&mut header)
            .map_err(|e| format!("Failed to read header: {}", e))?;

        // Read triangle count (4 bytes, little endian)
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)
            .map_err(|e| format!("Failed to read triangle count: {}", e))?;
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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "obj" => Self::from_obj(path),
            "stl" => Self::from_stl(path),
            _ => Err(format!("Unsupported file format: .{} (use .obj or .stl)", ext)),
        }
    }
}
