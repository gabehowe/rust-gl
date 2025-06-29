#version 460 core
layout (location = 0) in vec3 aPos;

uniform mat4 model;
uniform float time;

out VS_OUT {
    vec3 FragPos;
} vs_out;
layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

void main()
{
    float aspect = projection[1][1] / projection[0][0];
    vec3 pos = vec3(aPos.x / aspect, aPos.y, aPos.z);
    mat4 correct_aspect = mat4(
    1. / aspect, 0., 0., 0.,
    0., 1., 0., 0.,
    0., 0., 1., 0.,
    0., 0., 0., 1.
    );
    vs_out.FragPos = vec3(model * vec4(aPos, 1.0));
    gl_Position = correct_aspect * model * vec4(aPos, 1.0);
}
