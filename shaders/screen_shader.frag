#version 460 core
out vec4 FragColor;

in VS_OUT {
    vec3 Normal;
    vec3 FragPos;
    float Time;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

struct Cell {
    ivec2 box;
    vec2 pos;
};
uniform sampler2D screen;

#define resolution vec2(1778, 1000)
#define PI 3.14159265358979393
void main() {
    FragColor = vec4(texture(screen, gl_FragCoord),0);
}
