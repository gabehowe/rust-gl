#version 460 core
layout (triangles) in;
layout (triangle_strip, max_vertices = 3) out;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};
in VS_OUT {
    vec3 Normal;
    vec3 FragPos;
} gs_in[];

out GS_OUT {
    vec3 Normal;
    vec3 FragPos;
} gs_out;

uniform mat4 model;


void main() {
    for (int i = 0; i < 3; i++){
        gs_out.FragPos = gs_in[i].FragPos;
//        gl_Position = vec4(gl_in[i].gl_Position.x, gl_in[i].gl_Position.y, n, 1.0);
        gl_Position = gl_in[i].gl_Position;
        gs_out.Normal = gs_in[i].Normal;
        EmitVertex();
    }
    EndPrimitive();
}