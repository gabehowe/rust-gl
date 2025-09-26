use crate::derive_transformable;
use crate::glutil::{BufferObject, GLBuffer, GLObject, Vaa, VertexArrayObject};
use crate::shader::{FromVertex, NarrowingMaterial, SetValue, Shader, ShaderManager, ShaderPtr};
use crate::transformation::{Transform, Transformable};
use crate::util::find_gl_error;
use cgmath::{Vector2, Vector3};
use gl::types::{GLenum, GLuint};
use gl::{ARRAY_BUFFER, FLOAT, STATIC_DRAW, TRIANGLES, TRIANGLE_FAN, UNSIGNED_INT};
use itertools::Itertools;
use obj::raw::{parse_mtl, parse_obj};
use obj::{FromRawVertex, TexturedVertex};
use std::any::Any;
use std::error::Error;
use std::ffi::{c_float, c_uint};
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null;

pub trait Render: Transformable {
    /// Renders the object using the given shader.
    /// # Errors
    /// If the rendering fails, it will return a `Box<dyn Error>`.
    fn render(&mut self, shader_override: Option<ShaderPtr>) -> Result<(), Box<dyn Error>>;
    fn is(&self) -> bool;
    fn set_is(&mut self, val: bool);
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Transformable for InstancedObject {
    fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.transforms.iter_mut().for_each(|v| v.scale(x, y, z));
    }

    fn uniform_scale(&mut self, scale: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.uniform_scale(scale));
    }

    fn rotate(&mut self, x: f32, y: f32, z: f32) {
        self.transforms.iter_mut().for_each(|v| v.rotate(x, y, z));
    }

    fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.translate(x, y, z));
    }

    fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.set_scale(x, y, z));
    }

    fn set_uniform_scale(&mut self, scale: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.set_uniform_scale(scale));
    }

    fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.set_rotation(x, y, z));
    }

    fn set_translation(&mut self, x: f32, y: f32, z: f32) {
        self.transforms
            .iter_mut()
            .for_each(|v| v.set_translation(x, y, z));
    }
}

impl Render for InstancedObject {
    fn render(&mut self, _: Option<ShaderPtr>) -> Result<(), Box<dyn Error>> {
        if self.transforms.is_empty() {
            return Ok(());
        }
        self.buffer_data()?;

        let mut shader = self.shader.borrow_mut();
        shader.use_();
        shader.update().expect("Shader should update.");

        unsafe {
            self.mesh.vertex_array.bind();
            gl::DrawElementsInstanced(
                self.draw_type,
                i32::try_from(self.mesh.vertices.len())?,
                UNSIGNED_INT,
                null(),
                i32::try_from(self.transforms.len())?,
            );
            self.mesh.vertex_array.unbind();
        }

        Ok(())
    }

    fn is(&self) -> bool {
        self.is
    }

    fn set_is(&mut self, val: bool) {
        self.is = val;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub struct InstancedObject {
    mesh: MeshData,
    transforms: Vec<Transform>,
    colors: Vec<[f32; 4]>,
    shader: ShaderPtr,
    is: bool,
    draw_type: GLenum,
}
impl InstancedObject {
    /// # Panics
    /// If the mesh data cannot be initialized.
    pub fn new(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Option<Vec<Vector3<f32>>>,
        shader: &ShaderPtr,
        transforms: Vec<Transform>,
        colors: Vec<[f32; 4]>,
    ) -> Self {
        let mut ret = Self {
            mesh: MeshData::new(vertices, indices, normals, None),
            shader: shader.clone(),
            transforms,
            colors,
            is: true,
            draw_type: TRIANGLE_FAN,
        };
        ret.mesh
            .vertex_array
            .vbos
            .push(BufferObject::new(ARRAY_BUFFER));
        ret.mesh
            .vertex_array
            .vbos
            .push(BufferObject::new(ARRAY_BUFFER));
        // ret.mesh.init().expect("Failed to initialize.");
        unsafe {
            let vertex_data = ret.mesh.build_vertex_data();
            let structure = vec![
                Vaa::new(FLOAT, 3, 0), // Pos
                Vaa::new(FLOAT, 4, 1), // Matrix row 1
                Vaa::new(FLOAT, 4, 1), // Matrix row 2
                Vaa::new(FLOAT, 4, 1), // Matrix row 3
                Vaa::new(FLOAT, 4, 1), // Matrix row 4
                Vaa::new(FLOAT, 4, 2), // Color
            ];
            ret.mesh
                .vertex_array
                .generate()
                .expect("Failed to generate VAO");
            ret.mesh.vertex_array.vbos[0]
                .buffer_data(vertex_data.as_slice(), STATIC_DRAW)
                .expect("Failed to buffer vertex data");
            ret.mesh
                .vertex_array
                .ebo
                .buffer_data(ret.mesh.indices.as_slice(), STATIC_DRAW)
                .expect("Failed to buffer index data");

            ret.buffer_data().expect("Data should have buffered.");

            ret.mesh
                .vertex_array
                .configure(structure)
                .expect("couldn't configure");

            // Set up instancing divisors
            for i in 1..8 {
                gl::VertexArrayBindingDivisor(ret.mesh.vertex_array.id, i, 1); // Transform matrices
            }
            find_gl_error().expect("Failed to set up instancing divisors!");
        }
        ret
    }

    /// # Errors
    /// If the data cannot be buffered.
    fn buffer_data(&mut self) -> Result<(), Box<dyn Error>> {
        // Buffer transform matrices
        let mats = self.transforms.iter().map(Transform::mat).collect_vec();
        self.mesh.vertex_array.vbos[1]
            .buffer_data(mats.as_slice(), STATIC_DRAW)
            .map_err(|_| "Failed to buffer transform data")?;

        // Buffer colors
        self.mesh.vertex_array.vbos[2]
            .buffer_data(self.colors.as_slice(), STATIC_DRAW)
            .map_err(|_| "Failed to buffer color data")?;
        Ok(())
    }
    pub fn set_data(&mut self, transforms: Vec<Transform>, colors: Vec<[f32; 4]>) {
        self.transforms = transforms;
        self.colors = colors;
    }

    pub const fn set_draw_type(&mut self, draw_type: GLenum) {
        self.draw_type = draw_type;
    }
}
pub struct MeshData {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u32>,
    pub vertex_array: VertexArrayObject,
    pub tex_coords: Option<Vec<Vector2<f32>>>, // TODO: use general vertex attribs or some type of builder instead of explicitly supporting only these two.
    pub normals: Option<Vec<Vector3<f32>>>,
}
impl MeshData {
    #[must_use] pub fn new(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Option<Vec<Vector3<f32>>>,
        tex_coords: Option<Vec<Vector2<f32>>>,
    ) -> Self {
        Self {
            vertices,
            indices,
            tex_coords,
            normals,
            vertex_array: VertexArrayObject::new(),
        }
    }
    /// Initializes the mesh data by generating the vertex array object and buffering the vertex data.
    /// # Errors
    /// If the OpenGL function fails, it will return a `Box<dyn Error>`.
    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let vertex_data = self.build_vertex_data();
        let mut structure = vec![Vaa::new(FLOAT, 3, 0)];
        if self.normals.is_some() {
            structure.push(Vaa::new(FLOAT, 3, 0));
        }
        if self.tex_coords.is_some() {
            structure.push(Vaa::new(FLOAT, 3, 0));
        }
        self.vertex_array.generate()?;
        self.vertex_array.vbos[0].buffer_data(vertex_data.as_slice(), STATIC_DRAW)?;
        self.vertex_array
            .ebo
            .buffer_data(self.indices.as_slice(), STATIC_DRAW)?;
        self.vertex_array.configure(structure)?;
        Ok(())
    }

    fn build_vertex_data(&self) -> Vec<f32> {
        let mut vertex_data = Vec::new();
        for i in 0..self.vertices.len() {
            vertex_data.push(self.vertices[i].x);
            vertex_data.push(self.vertices[i].y);
            vertex_data.push(self.vertices[i].z);
            if let Some(d) = &self.normals {
                vertex_data.push(d[i].x);
                vertex_data.push(d[i].y);
                vertex_data.push(d[i].z);
            }
            if let Some(d) = &self.tex_coords {
                vertex_data.push(d[i].x);
                vertex_data.push(d[i].y);
                vertex_data.push(0.0);
            }
        }
        vertex_data
    }
}

pub struct Renderable {
    pub mesh_data: MeshData,
    pub transform: Transform,
    pub shader: ShaderPtr,
    pub draw_type: GLenum,
    is: bool,
}
impl Renderable {
    /// Creates a new Renderable with the given vertices, indices, normals and shader.
    /// # Panics
    /// If the mesh data cannot be initialized.
    pub(crate) fn new_with_tex(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        tex_coords: Vec<Vector2<f32>>,
        shader: &ShaderPtr,
    ) -> Self {
        let mut ret = Self::only_data(vertices, indices, Some(normals), shader);
        ret.mesh_data.tex_coords = Some(tex_coords);
        ret.mesh_data
            .init()
            .expect("Failed to initialize mesh data.");
        ret
    }
    /// # Panics
    /// If the mesh data cannot be initialized.
    pub fn new(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Option<Vec<Vector3<f32>>>,
        shader: &ShaderPtr,
    ) -> Self {
        let mut ret = Self::only_data(vertices, indices, normals, shader);
        ret.mesh_data
            .init()
            .expect("Failed to initialize mesh data.");
        ret
    }
    fn only_data(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Option<Vec<Vector3<f32>>>,
        shader: &ShaderPtr,
    ) -> Self {
        Self {
            mesh_data: MeshData::new(vertices, indices, normals, None),
            shader: shader.clone(),
            transform: Transform::default(),
            draw_type: TRIANGLES,
            is: true,
        }
    }

    /// # Errors
    /// If the object cannot be created from the given path or shader path.
    pub fn from_obj(
        path: &str,
        shaderpath: &str,
        manager: &mut ShaderManager,
    ) -> Result<Self, Box<dyn Error>> {
        let path_dir = Path::new(path).parent().ok_or("Invalid path")?;
        let input = BufReader::new(File::open(path).map_err(|e| format!("Couldn't open file {path}: {e}"))?);
        let obj = parse_obj(input).map_err(|_| "Couldn't parse obj!")?;
        // let parsed_obj: Obj<TexturedVertex> = Obj::new(obj).expect("Jimbo jones the fourth");
        let (vertices, indices) = FromRawVertex::<u32>::process(
            obj.positions,
            obj.normals,
            obj.tex_coords.clone(),
            obj.polygons,
        )
        .map_err(|_| "Couldn't process vertices")?;
        let path_str = path_dir.to_str().ok_or("Invalid path")?;
        let raw_mtl = parse_mtl(BufReader::new(
            File::open((path_str.to_owned()) + "/" + &obj.material_libraries[0])
                .map_err(|_| {
                    format!(
                        "Cannot find file {}",
                        path_str.to_owned()
                            + "/"
                            + &obj.material_libraries[0]
                    )
                })?,
        ))
        .map_err(|_| "Couldn't parse mtl!")?;
        let mat =
            NarrowingMaterial::from_obj_mtl(&raw_mtl.materials.get("Material.001").ok_or("Couldn't get material")?.clone());
        let new_shader = mat.with_path(shaderpath)?;

        // let new_shader = Shader::load_from_path("shaders/comp_base_shader");
        Ok(Self::new_with_tex(
            vertices.iter().map(Vector3::from_vertex).collect(),
            indices,
            vertices.iter().map(Vector3::from_vertex).collect(),
            vertices
                .iter()
                .map(|x: &TexturedVertex| Vector2::new(x.texture[0], x.texture[1]))
                .collect(),
            &manager.register(new_shader),
        ))
    }
}
impl Render for Renderable {
    fn render(&mut self, shader_override: Option<ShaderPtr>) -> Result<(), Box<dyn Error>> {
        if !self.is {
            return Ok(());
        }
        let model = self.transform.mat();
        let mut shader = shader_override
            .as_ref()
            .unwrap_or(&self.shader)
            .borrow_mut();
        shader.use_();
        shader.update().expect("Shader failed to update.");
        shader.set(model, "model").expect("Couldn't set shader");

        unsafe {
            // gl::BindVertexArray(self.mesh_data.vertex_array);
            self.mesh_data.vertex_array.bind();
            find_gl_error()?;
            gl::DrawElements(
                self.draw_type,
                i32::try_from(self.mesh_data.indices.len() * size_of::<GLuint>())?,
                UNSIGNED_INT,
                null(),
            );
            self.mesh_data.vertex_array.unbind();
            // gl::BindVertexArray(0); // Cleanup
        }
        Shader::clear_shader();
        Ok(())
    }

    fn is(&self) -> bool {
        self.is
    }

    fn set_is(&mut self, val: bool) {
        self.is = val;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
derive_transformable!(Renderable);

pub struct RenderableGroup {
    renderables: Vec<Renderable>,
    is: bool,
}
impl RenderableGroup {
    /// Creates a new `RenderableGroup` with the given renderables.
    /// # Errors
    /// If the renderables cannot be created from the given path or shader path.
    pub fn from_gltf(
        path: &str,
        shaderpath: &str,
        shader_manager: &mut ShaderManager,
    ) -> Result<Self, Box<dyn Error>> {
        let mut ancestors = Path::new(path).ancestors();
        let mut base = "";
        ancestors.next();
        if let Some(root) = ancestors.next() {
            base = root.to_str().ok_or("Should be a string.")?;
        }

        let (document, buffers, images) = gltf::import(path)?;

        let mut renderables: Vec<Renderable> = Vec::new();
        let mut materials: Vec<ShaderPtr> = Vec::new();
        for i in document.materials() {
            let mat = NarrowingMaterial::from_gltf_mtl(&i, &images, &buffers, base)?;
            materials.push(shader_manager.register(mat.with_path(shaderpath)?));
        }
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let vertices: Vec<Vector3<c_float>> =
                    reader.read_positions().ok_or("Couldn't read positions")?.map_into().collect();
                let indices: Vec<c_uint> = reader.read_indices().ok_or("Couldn't read indices")?.into_u32().collect();
                let tex_coords: Vec<Vector2<c_float>> = reader
                    .read_tex_coords(0)
                    .ok_or("Couldn't read texture coordinates")?
                    .into_f32()
                    .map_into()
                    .collect(); //TODO: add multiple sets
                let normals: Vec<Vector3<c_float>> =
                    reader.read_normals().ok_or("Couldn't read normals")?.map_into().collect();
                let material = materials[primitive.material().index().ok_or("couldn't read index")?].clone();

                renderables.push(Renderable::new_with_tex(
                    vertices, indices, normals, tex_coords, &material,
                ));
            }
        }
        Ok(Self {
            renderables,
            is: true,
        })
    }
    #[allow(clippy::cast_precision_loss)]
    #[must_use] pub fn create_grid(
        width: u32,
        length: u32,
        scale: f32,
        pos: Vector2<f32>,
    ) -> (Vec<Vector3<f32>>, Vec<u32>, Vec<Vector3<f32>>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut offset = 0;

        for i in 0..width {
            for j in 0..length {
                vertices.push(Vector3::new(
                    (i as f32).mul_add(scale, pos.x),
                    0.0,
                    (j as f32).mul_add(scale, pos.y),
                ));
                normals.push(Vector3::new(0.0, 1.0, 0.0));
                if i != 0 && j != 0 {
                    indices.push(offset - length - 1);
                    indices.push(offset - length);
                    indices.push(offset);
                    indices.push(offset - 1);
                    indices.push(offset - length - 1);
                    indices.push(offset);
                }
                offset += 1;
            }
        }
        (vertices, indices, normals)
    }
}

impl Render for RenderableGroup {
    fn render(&mut self, shader_override: Option<ShaderPtr>) -> Result<(), Box<dyn Error>> {
        if !self.is {
            return Ok(());
        }
        self.renderables.iter_mut().try_for_each(|r| {
            let shader = shader_override.as_ref().map(std::clone::Clone::clone);
            r.render(shader)
        })
    }

    fn is(&self) -> bool {
        self.is
    }

    fn set_is(&mut self, val: bool) {
        self.is = val;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl Transformable for RenderableGroup {
    fn scale(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].scale(x, y, z);
        }
    }
    fn uniform_scale(&mut self, scale: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].uniform_scale(scale);
        }
    }
    fn rotate(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].rotate(x, y, z);
        }
    }
    fn translate(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].translate(x, y, z);
        }
    }

    fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].set_scale(x, y, z);
        }
    }

    fn set_uniform_scale(&mut self, scale: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].set_uniform_scale(scale);
        }
    }

    fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].set_rotation(x, y, z);
        }
    }

    fn set_translation(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..self.renderables.len() {
            self.renderables[i].set_translation(x, y, z);
        }
    }
}
