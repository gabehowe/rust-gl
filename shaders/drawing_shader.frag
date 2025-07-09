#version 460 core
out vec4 FragColor;
in VS_OUT {
    vec3 FragPos;
    vec4 Color;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

void main() {
    FragColor = fs_in.Color;
//        FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
