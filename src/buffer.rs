use std::collections::HashMap;
use std::iter::Map;
use std::mem::size_of;
use gl::types::{GLsizei, GLuint};

struct VertexArrayAttrib {
    size: u16,
    _type: gl::types::GLenum,
    type_size: usize,
}

struct VertexArray {
    id: GLuint,
    attributes: Vec<VertexArrayAttrib>,
    stride: GLsizei,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut id = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        VertexArray {
            id,
            attributes: Vec::new(),
            stride: 0,
        }
    }
    pub unsafe fn add_vertex_attrib<T>(&mut self, size: u16, _type: gl::types::GLenum) {
        self.attributes.push(VertexArrayAttrib {
            size,
            _type,
            type_size: size_of::<T>(),
        });
        self.stride += size as GLsizei * size_of::<T>() as GLsizei;
    }

    pub unsafe fn build_vertex_attribs(&mut self) {
        let mut offset = 0i32;
        for i in 0..self.attributes.len() {
            let vert_array_attrib = &self.attributes[i];
            gl::EnableVertexAttribArray(i as GLuint);
            gl::VertexAttribPointer(
                i as GLuint,
                vert_array_attrib.size as i32,
                vert_array_attrib._type,
                gl::FALSE,
                self.stride,
                (offset as GLuint) as *const _,
            );
            offset += vert_array_attrib.size as GLsizei * vert_array_attrib.type_size as GLsizei;
        }
    }
}

pub(crate) struct ShaderMemoryManager {
    vertex_arrays: HashMap<u16, VertexArray>,
}

impl ShaderMemoryManager {
    pub fn new() -> ShaderMemoryManager {
        ShaderMemoryManager {
            vertex_arrays: HashMap::new()
        }
    }
    pub fn add_vertex_array(&mut self) -> u16 {
        let vertex_array = VertexArray::new();
        self.vertex_arrays.insert(vertex_array.id as u16, vertex_array);
        return self.vertex_arrays.iter().last().unwrap().0.clone();
    }
}