/*
Store an image in a matrix
Write the matrix to a texture
*/
#![feature(generic_const_exprs)]
#![allow(incomplete_features, unused)]

use crate::shader::Shader;
use noise::Vector2;
use std::cmp::min_by;
use cgmath::num_traits::AsPrimitive;

pub struct Draw {
    matrix: Vec<u8>,
    width: usize,
    height: usize,
    size: usize,
}
impl Default for Draw {
    fn default() -> Self {
        Self::new(10, 10)
    }
}

impl Draw {
    pub fn new(width: usize, height: usize) -> Self {
        let mut ret = Draw {
            size: height * width * 4,
            height,
            width,
            matrix: Vec::with_capacity(height * width * 4),
        };
        ret.matrix.resize(height * width * 4, 0);
        ret
    }
    pub fn clear(&mut self) {
        self.matrix.clear()
    }
    pub fn fill(&mut self, color: [u8; 4]) {
        // todo!("Speed this up with some kind of vector extension or something. Lazy loading? ")
        for i in 0..self.size {
            self.matrix[i] = color[i % 4];
        }
    }
    pub fn line(&mut self, p1: [usize; 2], p2: [usize; 2], color: [u8; 4], width: usize) {
        let lpoint = if p1[0] <= p2[0] { p1 } else { p2 };
        let rpoint = if lpoint == p1 { p2 } else { p1 };
        println!("{}", lpoint == p1);
        let lpoint: [i64; 2] = lpoint.map(usize::as_);
        let rpoint: [i64; 2] = rpoint.map(usize::as_);
        let length = rpoint[0] - lpoint[0];
        let slope = (rpoint[1] - lpoint[1]) as f64 / (length as f64);
        let mut y = ((lpoint[0]) as f64 + lpoint[1] as f64);
        for i in 0..length {
            y += slope;
            for v in -(width as i32)..(width as i32) {
                for c in 0..4 {
                    let xcoord = lpoint[0] + i + c;
                    let ycoord = y as i32 + v;
                    // println!("{} {}", xcoord, ycoord);
                    self.matrix[self.width * ycoord as usize + xcoord as usize] = color[c as usize];
                    // TODO: solve the edge case when y + v goes outside the bounds.
                }
            }
        }
    }
    pub fn circle(&mut self, center: [usize; 2], radius: usize, width: usize) {
        let mut eighth: Vec<[usize; 2]> = Vec::new();
        let mut x = radius as i32;
        let mut t1 = x / 16;
        let mut y = 0;
        while x >= y {
            eighth.push([x as usize, y as usize]);
            y += 1;
            t1 += y;
            let t2 = t1 - x;
            if (t2 >= 0) {
                t1 = t2;
                x -= 1;
            }
        }
        let mut pixels: Vec<([usize; 2], [u8; 4])> = Vec::new();
        for i in 0..eighth.len() {
            for x in [-1, 1] {
                for y in [-1, 1] {
                    let p = [
                        center[0] as i32 + eighth[i][0] as i32 * x,
                        center[1] as i32 + eighth[i][1] as i32 * y,
                    ];
                }
            }
        }
    }
    pub fn blit(&mut self, map: Vec<([usize; 2], [u8; 4])>) {
        for (pos, color) in map {
            for i in 0..4 {
                self.matrix[self.width * pos[1] + pos[0] + i] = color[i]
            }
        }
    }
    pub fn debug_print(&self) {
        for y in 0..self.height {
            for x in 0..self.width {
                print!("{:#.1} ", (self.matrix[self.width * y + x] as f64) / 255.)
            }
            println!()
        }
    }
    pub fn transfer(&self, texture: usize) {
        Shader::set_texture(texture, &self.matrix, self.width, self.height);
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        let oldmat = self.matrix.clone();
        let oldwidth = self.width;
        let oldheight = self.height;
        self.width = width;
        self.height = height;
        self.matrix.resize(height * width * 4, 0);
        self.matrix.fill(0);
        for i in 0..(oldwidth * oldheight) {
            self.matrix[i / oldwidth * self.width + i % oldwidth] = oldmat[i]
        }
    }
}
