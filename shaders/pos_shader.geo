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
    gs_out.Normal = gs_in[0].Normal;
    gs_out.FragPos = gs_in[0].FragPos;
    gl_Position = gl_in[0].gl_Position;
    EmitVertex();
    gs_out.Normal = gs_in[1].Normal;
    gs_out.FragPos = gs_in[1].FragPos;
    gl_Position = gl_in[1].gl_Position;
    EmitVertex();
    gs_out.Normal = gs_in[2].Normal;
    gs_out.FragPos = gs_in[2].FragPos;
    gl_Position = gl_in[2].gl_Position;
    EmitVertex();
    EndPrimitive();
}
