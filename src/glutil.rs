use std::string::String;
use crate::util::{find_gl_error, GLFunctionError};
use gl::types::{GLbyte, GLdouble, GLenum, GLfloat, GLint, GLshort, GLubyte, GLuint, GLushort};
use gl::{ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER};
use std::collections::HashMap;
use std::error::Error;
use std::ops::AddAssign;

pub trait GLObject {
    fn generate(&mut self) -> Result<(), Box<dyn Error>>;
    fn bind(&mut self);
    fn unbind(&mut self);
}
pub trait GLBuffer: GLObject {
    fn buffer_data<T: Sized>(&mut self, data: &[T], usage: GLenum) -> Result<(), GLFunctionError>;
}
/// Vertex Array Attribute
pub struct VAA {
    kind: GLenum,
    amount: u32,
    type_size: usize,
    mem_size: usize,
    vbo_index: u32,
}
impl VAA {
    pub fn new(kind: GLenum, amount: u32, vbo_index: u32) -> Self {
        let ts = VertexArrayObject::get_type_size(kind).expect("");
        VAA {
            kind,
            amount,
            type_size: ts,
            mem_size: ts * amount as usize,
            vbo_index,
        }
    }
    pub fn set_vbo_index(&mut self, ebo_index: u32) {
        self.vbo_index = ebo_index;
    }
}
pub struct VertexArrayObject {
    pub(crate) id: u32,
    generated: bool,
    bound: bool,
    structure: Vec<VAA>, // Length and data type
    pub ebo: BufferObject,
    pub vbos: Vec<BufferObject>,
}
impl GLObject for VertexArrayObject {
    fn generate(&mut self) -> Result<(), Box<dyn Error>> {
        find_gl_error()?;
        unsafe {
            gl::CreateVertexArrays(1, &mut self.id);
        }
        find_gl_error()?;
        self.generated = true;
        self.ebo.generate()?;
        self.vbos.iter_mut().try_for_each(|arg0| arg0.generate())?;
        unsafe {
            gl::VertexArrayElementBuffer(self.id, self.ebo.id);
        }
        find_gl_error().map_err(Box::from)
    }

    fn bind(&mut self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
        self.bound = true;
    }
    fn unbind(&mut self) {
        unsafe {
            gl::BindVertexArray(0);
        }
        self.bound = false;
    }
}
impl Default for VertexArrayObject {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexArrayObject {
    pub fn new() -> VertexArrayObject {
        VertexArrayObject {
            id: 0,
            generated: false,
            bound: false,
            structure: vec![],
            ebo: BufferObject::new(ELEMENT_ARRAY_BUFFER),
            vbos: vec![BufferObject::new(ARRAY_BUFFER)],
        }
    }
    pub(crate) fn configure(&mut self, structure: Vec<VAA>) -> Result<(), String> {
        self.structure = structure;
        // Enforce state
        if self.vbos.iter().any(|x| !x.buffered) {
            return Err("Some VBO has not yet been buffered!".to_owned());
        }

        let mut stride: HashMap<u32, u32> = HashMap::new();
        for i in &self.structure {
            stride.entry(i.vbo_index).or_insert(0);
            stride.get_mut(&i.vbo_index).unwrap().add_assign(i.mem_size as u32);
        }
        let mut pointer_offset: HashMap<u32, u32> = HashMap::new();
        unsafe {
            for i in 0..self.vbos.len() {
                gl::VertexArrayVertexBuffer(self.id, i as u32, self.vbos[i].id, 0, stride[&(i as u32)] as i32);
                pointer_offset.insert(i as u32, 0);
            }
        }
        for i in 0..self.structure.len() {
            unsafe {
                gl::EnableVertexArrayAttrib(self.id, i as u32);
                gl::VertexArrayAttribFormat(
                    self.id,
                    i as u32,
                    self.structure[i].amount as i32,
                    self.structure[i].kind,
                    false as u8,
                    pointer_offset[&self.structure[i].vbo_index],
                );
                gl::VertexArrayAttribBinding(self.id, i as GLuint, self.structure[i].vbo_index);
            }
            pointer_offset
                .get_mut(&self.structure[i].vbo_index)
                .unwrap()
                .add_assign(self.structure[i].mem_size as u32);
        }
        find_gl_error().map_err(|x| x.message)
    }

    fn get_type_size(type_enum: GLenum) -> Result<usize, String> {
        match type_enum {
            gl::FLOAT => Ok(size_of::<GLfloat>()),
            gl::UNSIGNED_INT => Ok(size_of::<GLuint>()),
            gl::BYTE => Ok(size_of::<GLbyte>()),
            gl::UNSIGNED_BYTE => Ok(size_of::<GLubyte>()),
            gl::SHORT => Ok(size_of::<GLshort>()),
            gl::UNSIGNED_SHORT => Ok(size_of::<GLushort>()),
            gl::INT => Ok(size_of::<GLint>()),
            gl::HALF_FLOAT => {
                todo!("Support half float.")
            }
            gl::DOUBLE => Ok(size_of::<GLdouble>()),
            _ => Err("Invalid type!".to_owned()),
        }
    }
}

pub struct BufferObject {
    pub(crate) id: u32,
    generated: bool,
    bound: bool,
    buffered: bool,
    kind: GLenum,
}
impl BufferObject {
    pub(crate) fn new(kind: GLenum) -> BufferObject {
        BufferObject {
            id: 0,
            generated: false,
            bound: false,
            buffered: false,
            kind,
        }
    }
}
impl GLObject for BufferObject {
    /// Must be run with a currently bound Vertex Array Object
    fn generate(&mut self) -> Result<(), Box<(dyn Error)>> {
        unsafe {
            gl::CreateBuffers(1, &mut self.id);
        }
        self.generated = true;
        find_gl_error().map_err(Box::from)
    }
    fn bind(&mut self) {
        unsafe {
            gl::BindBuffer(self.kind, self.id);
        }
        self.bound = true;
    }
    fn unbind(&mut self) {
        unsafe {
            gl::BindBuffer(self.kind, 0);
        }
        self.bound = false;
    }
}
impl GLBuffer for BufferObject {
    fn buffer_data<T: Sized>(&mut self, data: &[T], usage: GLenum) -> Result<(), GLFunctionError> {
        self.buffered = true;
        unsafe {
            gl::NamedBufferData(
                self.id,
                size_of_val(data) as isize,
                data.as_ptr().cast(),
                usage,
            )
        }
        find_gl_error()
    }
}
