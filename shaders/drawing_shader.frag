#version 460 core
out vec4 FragColor;
uniform vec4 color;
in VS_OUT {
    vec3 FragPos;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

void main() {
    FragColor = color;
    //    FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
