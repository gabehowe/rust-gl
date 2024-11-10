#version 460 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

uniform mat4 model;
uniform float time;

out VS_OUT {
    vec3 Normal;
    vec3 FragPos;
    float Time;
} vs_out;

void main()
{
    vs_out.Normal = mat3(transpose(inverse(model))) * aNormal;
    vs_out.FragPos = vec3(vec4(aPos, 1.0) * model);
    vs_out.Time = time;

    gl_Position = model * vec4(aPos.xyz, 1.0);
}
