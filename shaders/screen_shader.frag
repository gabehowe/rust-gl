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
    //    FragColor = vec4(gl_FragCoord.x/resolution.x, 0.0, gl_FragCoord.y/resolution.y,1.0);
    vec2 manipulativeOcds = gl_FragCoord.xy / resolution - vec2(0.5);
    float k = 2.0;
    float k1 = 0.0;
    float dist = distance(manipulativeOcds, vec2(0.0));
    vec2 new_vec = manipulativeOcds/(1+ k*dist*dist + k1*pow(dist,4)) + vec2(0.5);
    if (mod(new_vec.x * 100,5) < 0.1 ||mod(new_vec.y * 100,5) < 0.1 ){
        FragColor = vec4(new_vec, 0.0,1.0);
    }
    else {
        FragColor = texture(screen, (manipulativeOcds + vec2(0.5))/3);
    }
}
