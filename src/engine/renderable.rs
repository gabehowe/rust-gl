use std::collections::HashMap;
use std::ffi::{c_float, c_int, c_uint, CString};
use std::fs::File;
use std::io::BufReader;
use std::mem::{size_of, transmute};
use std::ops::Index;
use std::os::raw::c_void;
use std::path::Path;
use std::ptr::null;

use cgmath::{Array, Euler, Matrix, Matrix4, One, Rad, Vector2, Vector3, Vector4, Zero};
use cgmath::num_traits::ToPrimitive;
use gl::{ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, FALSE, FLOAT, FRAGMENT_SHADER, STATIC_DRAW, TEXTURE_2D, TEXTURE_WRAP_S, TEXTURE_WRAP_T, TRIANGLES, UNSIGNED_INT, VERTEX_SHADER};
use gl::types::{GLenum, GLfloat, GLint, GLsizei, GLuint};
use glfw::ffi::glfwGetTime;
use image::open;
use obj::{FromRawVertex, TexturedVertex, Vertex};
use obj::raw::{parse_mtl, parse_obj};
use obj::raw::material::{Material, MtlColor};

use crate::engine::transformation::Transformation;
use crate::engine::util::load_file;

trait FromVertex {
    fn from_vertex(vertex: &Vertex) -> Self;
    fn from_tex_vertex(vertex: &TexturedVertex) -> Self;
}

impl FromVertex for Vector3<f32> {
    fn from_vertex(vertex: &Vertex) -> Self {
        Vector3::new(vertex.position[0], vertex.position[1], vertex.position[2])
    }
    fn from_tex_vertex(vertex: &TexturedVertex) -> Self {
        Vector3::new(vertex.position[0], vertex.position[1], vertex.position[2])
    }
}

fn from_color(color: Option<MtlColor>) -> Vec<f32> {
    if let Some(MtlColor::Rgb(r, g, b)) = color {
        return vec![r, g, b];
    }
    return Vec::new();
}
// A struct to build shaders depending on the options provided in the material.

pub struct Shader {
    path: String,
    vert: u32,
    frag: u32,
    program: u32,
    geo: u32,
    optionals: i32,
    textures: HashMap<String, u32>,
    vector_values: HashMap<String, Vec<f32>>,
    values: HashMap<String, f32>,
}

impl Shader {
    pub unsafe fn load_from_path(path: &str) -> Shader {
        let mut vert_string = path.to_owned().clone();
        vert_string.push_str(".vert");

        let mut frag_string = path.to_owned().clone();
        frag_string.push_str(".frag");
        let vert_source = load_file(vert_string);
        let frag_source = load_file(frag_string);

        let geo_string = format!("{}.geo", path);
        let mut geo_source = CString::new("").expect("Jimbo jones");
        if Path::new(&geo_string).exists() {
            geo_source = load_file(geo_string);
        }
        let mut ret = Shader {
            path: path.to_owned(),
            vert: gl::CreateShader(VERTEX_SHADER),
            frag: gl::CreateShader(FRAGMENT_SHADER),
            program: gl::CreateProgram(),
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::from([("ambient".to_owned(), vec![0.; 3]), ("diffuse".to_owned(), vec![0.; 3]), ("specular".to_owned(), vec![0.; 3]), ("emissive".to_owned(), vec![0.; 3])]),
            values: HashMap::new(),
        };
        ret.compile(vert_source, frag_source, geo_source);
        return ret;
    }
    pub unsafe fn load_from_mtl(mtl: Material, mtl_dir: &str, base_path: &str) -> Shader {
        let mut ret = Shader {
            path: base_path.to_owned(),
            vert: gl::CreateShader(VERTEX_SHADER),
            frag: gl::CreateShader(FRAGMENT_SHADER),
            program: gl::CreateProgram(),
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::new(),
            values: Default::default(),
        };
        if mtl.ambient.is_some() {
            ret.vector_values.insert("ambient".to_owned(), from_color(mtl.ambient));
        }
        if mtl.diffuse.is_some() {
            ret.vector_values.insert("diffuse".to_owned(), from_color(mtl.diffuse));
        }
        if mtl.specular.is_some() {
            ret.vector_values.insert("specular".to_owned(), from_color(mtl.specular));
        }
        if mtl.emissive.is_some() {
            ret.vector_values.insert("emissive".to_owned(), from_color(mtl.emissive));
        }
        if mtl.optical_density.is_some() {
            let ior = mtl.optical_density.unwrap();
            // let specular = ((ior-1.0)/(ior+1.0)).powf(2.0)/0.08;
            let specular = 256.0;
            ret.values.insert("specular_exponent".to_owned(), specular);
        }

        if mtl.diffuse_map.is_some() {
            let path = mtl.diffuse_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert("diffuse".to_owned(), Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()));
        }

        if mtl.ambient_map.is_some() {
            let path = mtl.ambient_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert("ambient".to_owned(), Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()));
        }

        if mtl.specular_map.is_some() {
            let path = mtl.specular_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert("specular".to_owned(), Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()));
        }

        if mtl.emissive_map.is_some() {
            let path = mtl.emissive_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert("emissive".to_owned(), Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()));
        }
        let mut vert_string = base_path.to_owned().clone();
        vert_string.push_str(".vert");
        let mut vert_source = load_file(vert_string).to_str().unwrap().to_owned();
        let mut frag_string = base_path.to_owned().clone();
        frag_string.push_str(".frag");
        let mut frag_source = load_file(frag_string).to_str().unwrap().to_owned();

        let mut locations = "layout (location = 0) in vec3 aPos;\nlayout (location = 1) in vec3 aNormal;\n".to_owned();
        let mut passthroughs = "";
        let mut outs = "vec3 Normal;\nvec3 FragPos;\n".to_owned();
        let uniforms = "uniform mat4 model;";
        let std140s = "layout (std140) uniform Matrices {vec3 cameraPos;\nmat4 view;\nmat4 projection;\n};";
        if mtl.ambient_map.clone().is_some() || mtl.diffuse_map.clone().is_some() || mtl.specular_map.clone().is_some() || mtl.emissive_map.is_some() {
            locations += "layout (location = 2) in vec2 aTexCoord;";
            passthroughs = "vs_out.TexCoord = aTexCoord;";
            outs += "vec2 TexCoord;";
        }
        vert_source = vert_source.replace("//T: LOCATIONS", format!("{}", locations).as_str());
        vert_source = vert_source.replace("//T: PASSTHROUGHS", passthroughs);
        vert_source = vert_source.replace("//T: OUT", format!("out VS_OUT {{\n{}}} vs_out;", outs).as_str());
        vert_source = vert_source.replace("//T: UNIFORMS", &uniforms);
        vert_source = vert_source.replace("//T: STD140", &std140s);

        frag_source = frag_source.replace("//T: IN", format!("in VS_OUT {{\n{}}} fs_in;", outs).as_str());
        frag_source = frag_source.replace("//T: OUT", "out vec4 FragColor;");
        frag_source = frag_source.replace("//T: STD140", &std140s);
        let mut textures = "".to_owned();
        let mut uniforms = "".to_owned();
        let mut logic = "".to_owned();
        for i in vec!["ambient", "diffuse", "specular", "emissive"] {
            if ret.textures.contains_key(i) {
                logic.push_str("vec4 ");
                logic.push_str(i);
                logic.push_str(" = texture(");
                logic.push_str(i);
                logic.push_str(", fs_in.TexCoord);\n");
            } else {
                uniforms.push_str("uniform vec4 ");
                uniforms.push_str(i);
                uniforms.push_str(";\n");
            }
        }
        if ret.values.contains_key("specular_exponent") {
            uniforms.push_str("uniform float specular_exponent;\n");
        }
        for i in 0..ret.textures.len() {
            let texture_name = (&mut ret).textures.keys().nth(i).unwrap();
            let mut texture = format!("layout (binding={i}) uniform sampler2D ").to_owned();
            texture.push_str(&texture_name.clone());
            texture.push_str(";\n");
            textures.push_str(&texture);
        }
        frag_source = frag_source.replace("//T: TEXTURES", &textures);
        frag_source = frag_source.replace("//T: LOGIC", &logic);
        frag_source = frag_source.replace("//T: UNIFORMS", &uniforms);
        ret.compile(CString::new(vert_source).expect("Jimbo jones"), CString::new(frag_source).expect("Jimbo jones"), CString::new("").expect("Jimbo jones"));
        ret.use_shader();
        for (i, v) in ret.vector_values.clone() {
            let ov = v.clone();
            let vector = Vector4::new(ov[0], ov[1], ov[2], 1.0);
            let os = i.clone();
            // println!("{}", i);
            ret.set_vec4(vector, os.as_str());
        }
        for (i, v) in ret.values.clone() {
            let os = i.clone();
            let ov = v.clone();
            ret.set_float(ov, os.as_str());
        }

        return ret;
    }

    unsafe fn compile_subshader(&mut self, source: CString, id: u32) {
        gl::ShaderSource(id, 1, &source.as_ptr(), null());
        gl::CompileShader(id);
        let mut success = 0;
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            gl::GetShaderInfoLog(id, 1024, &mut log_len, v.as_mut_ptr().cast());
            v.set_len(log_len.try_into().unwrap());
            println!("{:?}", source);
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        gl::AttachShader(self.program, id);
    }
    pub unsafe fn compile(&mut self, vert_source: CString, frag_source: CString, geo_source: CString) {
        if geo_source.to_str().unwrap() != "" {
            self.geo = gl::CreateShader(gl::GEOMETRY_SHADER);
            self.compile_subshader(geo_source, self.geo);
        }
        println!("{:?}", self.geo);
        println!("{:?}", vert_source.to_str().unwrap().replace("\\n", "\r\n"));
        println!("{:?}", frag_source.to_str().unwrap().replace("\\n", "\r\n"));

        self.compile_subshader(vert_source, self.vert);
        self.compile_subshader(frag_source, self.frag);

        gl::LinkProgram(self.program);

        let mut success = 0;
        gl::GetProgramiv(self.program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            gl::GetProgramInfoLog(self.program, 1024, &mut log_len, v.as_mut_ptr().cast());
            v.set_len(log_len.try_into().unwrap());
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        // gl::DeleteProgram(self.vert);
        // gl::DeleteProgram(self.frag);
        self.check_optionals()
    }

    unsafe fn get_shader_error(&mut self) -> String {
        let mut v: Vec<u8> = Vec::with_capacity(1024);
        let mut log_len = 0_i32;
        gl::GetShaderInfoLog(self.frag, 1024, &mut log_len, v.as_mut_ptr().cast());
        v.set_len(log_len.try_into().unwrap());
        let ret_str = String::from_utf8(v).expect("Jimbo jones");
        return ret_str;
    }
    unsafe fn update(&mut self) {
        self.update_optionals()
    }
    pub unsafe fn use_shader(&mut self) {
        gl::UseProgram(self.program);
    }

    unsafe fn check_optionals(&mut self) {
        if self.get_uniform_location("time") != -1 {
            self.optionals |= 1;
        }
    }
    unsafe fn update_optionals(&mut self) {
        if self.optionals & 1 == 1 {
            self.set_float(glfwGetTime() as f32, "time")
        }
    }
    unsafe fn setup_textures() {
        gl::TexParameteri(TEXTURE_2D, TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(TEXTURE_2D, TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        // gl::GenTextures(1, &mut self.texture);
        // gl::BindTexture(TEXTURE_2D, self.texture);
    }
    unsafe fn load_texture(path: &str) -> u32 {
        let img = open(path).expect("Jimbo jones");
        let height = img.height();
        let width = img.width();
        let data = img.to_rgb8().into_raw();
        Self::create_texture(&data[0] as *const u8 as *const c_void, width as i32, height as i32)
    }
    unsafe fn create_texture(data: *const c_void, width: i32, height: i32) -> u32 {
        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(TEXTURE_2D, texture);
        Self::setup_textures();
        gl::TexImage2D(TEXTURE_2D, 0, gl::RGB as i32, width, height, 0, gl::RGB, gl::UNSIGNED_BYTE, data);
        // gl::GenerateMipmap(TEXTURE_2D);
        gl::BindTexture(TEXTURE_2D, 0);
        texture
    }
    unsafe fn use_textures(&mut self) {
        let vals = self.textures.values().cloned().collect::<Vec<u32>>();
        if vals.len() == 0 { return; }
        gl::BindTextures(0, self.textures.len() as GLsizei, vals.as_ptr());
    }

    pub unsafe fn bind_matrices(&mut self) {
        let block_name = CString::new("Matrices").unwrap();
        let cast = block_name.into_raw();
        let index = gl::GetUniformBlockIndex(self.program, cast.cast());
        gl::UniformBlockBinding(self.program, index, 0);
    }

    unsafe fn is_used(&mut self) -> bool {
        let mut is_used = 0;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut is_used);
        return is_used == self.program as i32;
    }

    unsafe fn get_uniform_location(&mut self, name: &str) -> GLint {
        let block_name = CString::new(name).unwrap();
        let casted = block_name.into_raw();
        let location = gl::GetUniformLocation(self.program, casted);
        if (location == -1) {}
        return location;
    }
    unsafe fn panic_if_error(&mut self, value: GLint, name: &str) {
        if value == -1 {
            let error = self.get_shader_error();
            panic!("Couldn't find location {}, {}, {}", name, error, self.path);
        }
    }
    pub unsafe fn set_int(&mut self, int: i32, name: &str) {
        let location = self.get_uniform_location(name);
        self.panic_if_error(location, name);
        gl::Uniform1i(location, int)
    }
    pub unsafe fn set_mat4(&mut self, matrix4: Matrix4<f32>, name: &str) {
        let location = self.get_uniform_location(name);
        self.panic_if_error(location, name);
        gl::UniformMatrix4fv(location, 1, FALSE, transmute(&matrix4[0][0]))
    }
    pub unsafe fn set_vec3(&mut self, vector3: Vector3<f32>, name: &str) {
        let location = self.get_uniform_location(name);
        self.panic_if_error(location, name);
        gl::Uniform3f(location, vector3.x, vector3.y, vector3.z)
    }

    pub unsafe fn set_vec4(&mut self, vector4: Vector4<f32>, name: &str) {
        let location = self.get_uniform_location(name);
        self.panic_if_error(location, name);
        gl::Uniform4f(location, vector4.x, vector4.y, vector4.z, vector4.w)
    }

    pub unsafe fn set_float(&mut self, float: f32, name: &str) {
        let location = self.get_uniform_location(name);
        self.panic_if_error(location, name);
        gl::Uniform1f(location, float)
    }
}

pub struct Renderable {
    pub(crate) vertices: Vec<Vector3<c_float>>,
    indices: Vec<c_uint>,
    pub shader: Shader,
    vertex_array: GLuint,
    vertex_buffer: GLuint,
    element_buffer: GLuint,
    rotation: Vector3<f32>,
    translation: Vector3<f32>,
    scale: Vector3<f32>,
    normals: Vec<Vector3<f32>>,
    tex_coords: Vec<Vector2<f32>>,
}

impl Renderable {
    pub(crate) fn new_with_tex(vertices: Vec<Vector3<f32>>, indices: Vec<u32>, normals: Vec<Vector3<f32>>, tex_coords: Vec<Vector2<f32>>, shader: Shader) -> Renderable {
        let mut ret = Renderable {
            vertices,
            indices,
            shader,
            vertex_array: 0,
            vertex_buffer: 0,
            element_buffer: 0,
            rotation: Vector3::zero(),
            translation: Vector3::zero(),
            scale: Vector3::new(1., 1., 1.),
            normals,
            tex_coords,
        };
        unsafe {
            ret.gen_buffers();
            gl::BindVertexArray(ret.vertex_array);

            ret.init_array_buffers();

            ret.gen_vertex_attrib_arrays();

            gl::BindBuffer(ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        return ret;
    }

    pub(crate) fn new(vertices: Vec<Vector3<f32>>, indices: Vec<u32>, normals: Vec<Vector3<f32>>, shader: Shader) -> Renderable {
        let mut ret = Renderable {
            vertices,
            indices,
            shader,
            vertex_array: 0,
            vertex_buffer: 0,
            element_buffer: 0,
            rotation: Vector3::zero(),
            translation: Vector3::zero(),
            scale: Vector3::new(1., 1., 1.),
            normals,
            tex_coords: Vec::new(),
        };
        unsafe {
            ret.gen_buffers();

            gl::BindVertexArray(ret.vertex_array);

            ret.init_array_buffers();

            ret.gen_vertex_attrib_arrays();
            //
            // gl::EnableVertexAttribArray(1);
            // gl::VertexAttribPointer(1, 3, FLOAT, FALSE, (6 * size_of::<GLfloat>()) as GLsizei, (3 * size_of::<GLfloat>()) as *const _);

            gl::BindBuffer(ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        return ret;
    }
    unsafe fn gen_vertex_attrib_arrays(&mut self) {
        let mut stride = 2 * (3 * size_of::<GLfloat>()) as GLsizei;
        if self.tex_coords.len() > 0 {
            stride = (2 * (3 * size_of::<GLfloat>()) + (2 * size_of::<GLfloat>())) as GLsizei;
        }
        gl::VertexAttribPointer(0, 3, FLOAT, FALSE, stride, 0 as *const _);
        gl::EnableVertexAttribArray(0);

        gl::VertexAttribPointer(1, 3, FLOAT, FALSE, stride, (3 * size_of::<GLfloat>()) as *const _);
        gl::EnableVertexAttribArray(1);

        if (self.tex_coords.len() > 0) {
            gl::VertexAttribPointer(2, 2, FLOAT, FALSE, stride, (6 * size_of::<GLfloat>()) as *const _);
            gl::EnableVertexAttribArray(2);
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
        let vertex_data = self.build_vertex_data();
        unsafe {
            gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
            let size = (vertex_data.len() * size_of::<GLfloat>()) as isize;
            gl::BufferData(
                ARRAY_BUFFER,
                size,
                transmute(&vertex_data[0]),
                STATIC_DRAW,
            );

            gl::BindBuffer(ELEMENT_ARRAY_BUFFER, self.element_buffer);
            gl::BufferData(
                ELEMENT_ARRAY_BUFFER,
                (self.indices.len() * size_of::<GLuint>()) as isize,
                transmute(&self.indices[0]),
                STATIC_DRAW,
            );
        }
    }
    fn build_vertex_data(&mut self) -> Vec<c_float> {
        let mut vertex_data = Vec::new();
        for i in 0..self.vertices.len() {
            vertex_data.push(self.vertices[i].x);
            vertex_data.push(self.vertices[i].y);
            vertex_data.push(self.vertices[i].z);

            vertex_data.push(self.normals[i].x);
            vertex_data.push(self.normals[i].y);
            vertex_data.push(self.normals[i].z);

            if self.tex_coords.len() > 0 {
                vertex_data.push(self.tex_coords[i].x);
                vertex_data.push(self.tex_coords[i].y);
            }
        }
        return vertex_data;
    }
    pub unsafe fn update_vertex_buffer(&mut self) {
        let vertex_data = self.build_vertex_data();
        gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
        gl::BufferSubData(ARRAY_BUFFER, 0, (vertex_data.len() * size_of::<GLfloat>()) as isize, transmute(&vertex_data[0]));
        gl::BindBuffer(ARRAY_BUFFER, 0);
    }
    unsafe fn enable_texture(&mut self) {
        // self.shader.setup_textures();
    }
    pub unsafe fn from_obj(path: &str, shaderpath: &str) -> Renderable {
        let path_dir = Path::new(path).parent().expect("Jimbo jones the second");
        let input = BufReader::new(File::open(path).expect("Jimbo jones again!"));
        let obj = parse_obj(input).expect("Jimb jones the third");
        // let parsed_obj: Obj<TexturedVertex> = Obj::new(obj).expect("Jimbo jones the fourth");
        let (vertices, indices) = FromRawVertex::<u32>::process(obj.positions, obj.normals, obj.tex_coords.clone(), obj.polygons).expect("");

        let raw_mtl = parse_mtl(BufReader::new(File::open((path_dir.to_str().unwrap().to_owned()) + "/" + &obj.material_libraries[0]).expect("Jimbo jones the fifth"))).expect("Jimbo jones the sixth");
        let new_shader = Shader::load_from_mtl(raw_mtl.materials.get("Material.001").expect("Jimbo jones the seventh").clone(), path_dir.to_str().unwrap(), shaderpath);
        // let new_shader = Shader::load_from_path("shaders/comp_base_shader");
        let mut ret = Renderable::new_with_tex(vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(), indices, vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(), vertices.iter().map(|x| Vector2::new(x.texture[0], x.texture[1])).collect(), new_shader);
        // let mut ret = Renderable::new(vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(), indices, vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(),  new_shader);
        return ret;
    }
    unsafe fn build_model(&mut self) -> Matrix4<f32> {
        let mut model = Matrix4::one();
        model = model.scale(self.scale.x, self.scale.y, self.scale.z);
        model = model * Matrix4::from_translation(self.translation);
        model = model * Matrix4::from(Euler::new(Rad(self.rotation.x), Rad(self.rotation.z), Rad(self.rotation.y)));
        // println!("{:?}", model);
        return model;
    }
    pub unsafe fn render(&mut self) {
        let model = self.build_model();
        self.shader.use_shader();
        self.shader.use_textures();
        self.shader.update();
        self.shader.set_mat4(model, "model");

        gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
        gl::BindVertexArray(self.vertex_array);
        gl::BindBuffer(ELEMENT_ARRAY_BUFFER, self.element_buffer);

        gl::DrawElements(
            TRIANGLES,
            (self.indices.len() * size_of::<GLuint>()) as GLsizei,
            UNSIGNED_INT,
            0 as *const c_void,
        );
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.scale.x *= x;
        self.scale.y *= y;
        self.scale.z *= z;
    }
    pub fn uniform_scale(&mut self, scale: f32) {
        self.scale *= scale;
    }
    pub fn rotate(&mut self, x: f32, y: f32, z: f32) {
        self.rotation += Vector3::new(x, y, z);
    }
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.translation += Vector3::new(x, y, z)
    }
}
