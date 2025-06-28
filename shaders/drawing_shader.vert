#version 460 core
layout (location = 0) in vec3 aPos;

uniform mat4 model;
uniform float time;

out VS_OUT {
    vec3 FragPos;
} vs_out;

void main()
{
    vs_out.FragPos = vec3(model * vec4(aPos, 1.0));
    gl_Position = model * vec4(aPos.xyz, 1.0);
}
