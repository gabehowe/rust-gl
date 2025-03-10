#version 430 core
layout (location = 0) in vec3 aPos;
//layout (location = 1) in vec3 aNormal;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};
uniform mat4 model;
void main() {
    gl_Position = (model * mat4(mat3(view))) * vec4(aPos, 1.0) * mat4(0.5,0,0,-0.89, 0,0.5,0,0.85, 0,0,0.5,0, 0,0,0,1);
}