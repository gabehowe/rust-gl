use crate::derive_transformable;
use crate::shader::{FromVertex, NarrowingMaterial, SetValue, Shader, ShaderManager, ShaderPtr};
use crate::transformation::{Transform, Transformable};
use crate::util::find_gl_error;
use cgmath::{Matrix4, Vector2, Vector3};
use gl::types::{GLenum, GLfloat, GLsizei, GLuint};
use gl::{ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, FALSE, FLOAT, STATIC_DRAW, TRIANGLES, UNSIGNED_INT};
use itertools::Itertools;
use obj::raw::{parse_mtl, parse_obj};
use obj::{FromRawVertex, TexturedVertex};
use std::error::Error;
use std::ffi::{c_float, c_uint};
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null;

pub trait Render: Transformable {
    fn render(&mut self, shader_override: Option<ShaderPtr>) -> Result<(), Box<dyn Error>>;
    fn is(&self) -> bool;
    fn set_is(&mut self, val: bool);
}
#[derive(Default)]
pub struct MeshData {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u32>,
    pub vertex_array: GLuint,
    pub vertex_buffer: GLuint,
    pub element_buffer: GLuint,
    pub tex_coords: Option<Vec<Vector2<f32>>>, // TODO: use general vertex attribs or some type of builder instead of explicitly supporting only these two.
    pub normals: Option<Vec<Vector3<f32>>>,
}
impl MeshData {
    pub fn init(&mut self) {
        self.gen_buffers();
        unsafe {
            gl::BindVertexArray(self.vertex_array);
        }
        self.init_array_buffers();
        self.gen_vertex_attrib_arrays();
    }
    fn gen_vertex_attrib_arrays(&mut self) {
        let mut stride = (3 * size_of::<GLfloat>()) as GLsizei; // Vertices
        let mut index = 0;
        let mut offset = 0;
        if self.normals.is_some() {
            // add stride for normals
            stride += 3 * size_of::<GLfloat>() as GLsizei;
        }
        if self.tex_coords.is_some() {
            // add stride for tex coords
            stride += (2 * size_of::<GLfloat>()) as GLsizei;
        }
        unsafe {
            gl::VertexAttribPointer(index, 3, FLOAT, FALSE, stride, null());
            gl::EnableVertexAttribArray(index);
        }
        offset += 3 * size_of::<GLfloat>() as GLsizei; // Next vertex attribute has this offset
        index += 1;
        if self.normals.is_some() {
            unsafe {
                gl::VertexAttribPointer(index, 3, FLOAT, FALSE, stride, offset as *const _);
                gl::EnableVertexAttribArray(index);
            }
            offset += 3 * size_of::<GLfloat>() as GLsizei; // Next vertex attribute has this offset
            index += 1;
        }
        if self.tex_coords.is_some() {
            unsafe {
                gl::VertexAttribPointer(
                    index, // Add one if normals exist
                    2,
                    FLOAT,
                    FALSE,
                    stride,
                    offset as *const _,
                );
                gl::EnableVertexAttribArray(index);
            }
            // the index and pointer must be incremented for additional vertex attributes.
            // index += 1;
            // pointer += 2 * size_of::<GLfloat>() as GLsizei;
        }
    }
    fn gen_buffers(&mut self) {
        unsafe {
            gl::GenBuffers(1, &mut self.vertex_buffer);
            gl::GenVertexArrays(1, &mut self.vertex_array);
            gl::GenBuffers(1, &mut self.element_buffer);
        }
    }
    fn init_array_buffers(&mut self) {
        let mut vertex_data = self.build_vertex_data();
        unsafe {
            gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
            let size = (vertex_data.len() * size_of::<GLfloat>()) as isize;
            gl::BufferData(
                ARRAY_BUFFER,
                size,
                vertex_data.as_mut_ptr().cast(),
                STATIC_DRAW,
            );

            gl::BindBuffer(ELEMENT_ARRAY_BUFFER, self.element_buffer);
            gl::BufferData(
                ELEMENT_ARRAY_BUFFER,
                (self.indices.len() * size_of::<GLuint>()) as isize,
                self.indices.as_mut_ptr().cast(),
                STATIC_DRAW,
            );
        }
    }
    fn build_vertex_data(&mut self) -> Vec<f32> {
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
            }
        }
        vertex_data
    }
    fn update_vertex_buffer(&mut self) {
        let vertex_data = self.build_vertex_data();
        unsafe {
            gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferSubData(
                ARRAY_BUFFER,
                0,
                (vertex_data.len() * size_of::<GLfloat>()) as isize,
                vertex_data.as_ptr().cast(),
            );
            gl::BindBuffer(ARRAY_BUFFER, 0);
        }
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
    pub(crate) fn new_with_tex(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        tex_coords: Vec<Vector2<f32>>,
        shader: &ShaderPtr,
    ) -> Renderable {
        let mut ret = Self::only_data(vertices, indices, normals, shader);
        ret.mesh_data.tex_coords = Some(tex_coords);
        ret.mesh_data.init();
        ret
    }

    pub fn new(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        shader: &ShaderPtr,
    ) -> Renderable {
        let mut ret = Self::only_data(vertices, indices, normals, shader);
        ret.mesh_data.init();
        ret
    }
    fn only_data(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        shader: &ShaderPtr,
    ) -> Renderable {
        Renderable {
            mesh_data: MeshData {
                vertices,
                indices,
                vertex_array: 0,
                vertex_buffer: 0,
                element_buffer: 0,
                normals: Some(normals),
                tex_coords: None,
            },
            shader: shader.clone(),
            transform: Transform::default(),
            draw_type: TRIANGLES,
            is: true,
        }
    }

    pub fn from_obj(
        path: &str,
        shaderpath: &str,
        manager: &mut ShaderManager,
    ) -> Result<Renderable, Box<dyn Error>> {
        let path_dir = Path::new(path).parent().expect("Jimbo jones the second");
        let input = BufReader::new(File::open(path).expect("Jimbo jones again!"));
        let obj = parse_obj(input).expect("Jimb jones the third");
        // let parsed_obj: Obj<TexturedVertex> = Obj::new(obj).expect("Jimbo jones the fourth");
        let (vertices, indices) = FromRawVertex::<u32>::process(
            obj.positions,
            obj.normals,
            obj.tex_coords.clone(),
            obj.polygons,
        )
        .map_err(|_| "Couldn't process vertices")?;

        let raw_mtl = parse_mtl(BufReader::new(
            File::open((path_dir.to_str().unwrap().to_owned()) + "/" + &obj.material_libraries[0])
                .map_err(|_| {
                    format!(
                        "Cannot find file {}",
                        path_dir.to_str().unwrap().parse::<String>().unwrap()
                            + "/"
                            + &obj.material_libraries[0]
                    )
                })?,
        ))
        .map_err(|_| "Couldn't parse mtl!")?;
        let mat =
            NarrowingMaterial::from_obj_mtl(raw_mtl.materials.get("Material.001").unwrap().clone());
        let new_shader = mat.with_path(shaderpath)?;

        // let new_shader = Shader::load_from_path("shaders/comp_base_shader");
        Ok(Renderable::new_with_tex(
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
        let mut shader = shader_override.as_ref().unwrap_or(&self.shader).borrow_mut();
        shader.use_();
        shader.update().expect("Shader failed to update.");
        shader.set(model, "model").expect("Couldn't set shader");

        unsafe {
            gl::BindVertexArray(self.mesh_data.vertex_array);
            find_gl_error()?;
            gl::DrawElements(
                self.draw_type,
                (self.mesh_data.indices.len() * size_of::<GLuint>()) as GLsizei,
                UNSIGNED_INT,
                null(),
            );
            gl::BindVertexArray(0); // Cleanup
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
}
derive_transformable!(Renderable);

pub struct RenderableGroup {
    renderables: Vec<Renderable>,
    is: bool,
}
impl RenderableGroup {
    pub fn from_gltf(
        path: &str,
        shaderpath: &str,
        shader_manager: &mut ShaderManager,
    ) -> Result<RenderableGroup, Box<dyn Error>> {
        let mut ancestors = Path::new(path).ancestors().to_owned();
        let mut base = "";
        ancestors.next();
        if let Some(root) = ancestors.next() {
            base = root.to_str().expect("Should be a string.");
        }

        let (document, buffers, images) = gltf::import(path)?;

        let mut renderables: Vec<Renderable> = Vec::new();
        let mut materials: Vec<ShaderPtr> = Vec::new();
        for i in document.materials() {
            let mat = NarrowingMaterial::from_gltf_mtl(i, &images, &buffers, base)?;
            materials.push(shader_manager.register(mat.with_path(shaderpath)?));
        }
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let vertices: Vec<Vector3<c_float>> =
                    reader.read_positions().unwrap().map_into().collect();
                let indices: Vec<c_uint> = reader.read_indices().unwrap().into_u32().collect();
                let tex_coords: Vec<Vector2<c_float>> = reader
                    .read_tex_coords(0)
                    .unwrap()
                    .into_f32()
                    .map_into()
                    .collect(); //TODO: add multiple sets
                let normals: Vec<Vector3<c_float>> =
                    reader.read_normals().unwrap().map_into().collect();
                let material = materials[primitive.material().index().unwrap()].clone();

                renderables.push(Renderable::new_with_tex(
                    vertices, indices, normals, tex_coords, &material,
                ));
            }
        }
        Ok(RenderableGroup {
            renderables,
            is: true,
        })
    }
    pub fn create_grid(
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
                    (i as f32 * scale) + pos.x,
                    0.0,
                    j as f32 * scale + pos.y,
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
            let shader = shader_override.as_ref().map(|x| x.clone());
            r.render(shader)
        })
    }

    fn is(&self) -> bool {
        self.is
    }

    fn set_is(&mut self, val: bool) {
        self.is = val;
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
}
