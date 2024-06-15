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

uniform float n1_g_inf;
uniform float n2_g_inf;

uniform float n1_f;
uniform float n2_f;
uniform float n3_f;

uniform float n1_p;
uniform float n2_p;
uniform float n3_p;

out VS_OUT {
    vec3 Normal;
    vec3 FragPos;
    float color;
} vs_out;

float rand(vec2 c) {
    return fract(sin(dot(c.xy, vec2(12.9898, 78.233))) * 43758.5453);
}

float noise(vec2 p, float freq) {
    float unit = 100 / freq;
    vec2 ij = floor(p / unit);
    vec2 xy = mod(p, unit) / unit;
    //xy = 3.*xy*xy-2.*xy*xy*xy;
    xy = .5 * (1. - cos(3.141592653589 * xy));
    float a = rand((ij + vec2(0., 0.)));
    float b = rand((ij + vec2(1., 0.)));
    float c = rand((ij + vec2(0., 1.)));
    float d = rand((ij + vec2(1., 1.)));
    float x1 = mix(a, b, xy.x);
    float x2 = mix(c, d, xy.x);
    return mix(x1, x2, xy.y);
}

float pNoise(vec2 p, int res, float f, float persistance) {
    float n = 0.;
    float normK = 0.;
    float amp = 5.;
    int iCount = 0;
    for (int i = 0; i < 50; i++) {
        n += amp * noise(p, f);
        f *= 2.;
        normK += amp;
        amp *= persistance;
        if (iCount == res) break;
        iCount++;
    }
    float nf = n / normK;
    return nf * nf * nf * nf;
}
vec2 gradient(float initial, vec2 off_pos, vec2 p, int res, float f, float persistence) {
    float h = 0.01;
    float differentX = (pNoise(off_pos + vec2(h, 0), res, f, persistence));
    float differentY = (pNoise(off_pos + vec2(0, h), res, f, persistence));
    float dx = (differentX - initial) / h;
    float dy = (differentY - initial) / h;
    return vec2(dx, dy);
}
float mag(vec2 v) {
    return sqrt(v.x * v.x + v.y * v.y);
}

void main() {

    float offset = 1;
    vec2 off_pos = vec2(4.0 * aPos.xz + offset);
    float n = pNoise(vec2(off_pos), 1000, n1_f, n1_p);
    float n1_g = mag(gradient(n, off_pos, vec2(aPos.xz), 1000, n1_f, n1_p));

    float n2 = (pNoise(vec2(off_pos), 1000, n2_f, n2_p) * 1) * (1 / (.9 + n1_g_inf * n1_g));
    float n2_g = mag(gradient(n2, off_pos + 5000, vec2(aPos.xz), 100, n2_f, n2_p));

    float n3 = (pNoise(vec2(4.0 * aPos.xz + time), 1000, n3_f, n3_p) * 1) * (1 / (1 + n2_g_inf * (n2_g + n1_g)));
    float n3_g = mag(gradient(n3, off_pos - 5000, vec2(aPos.xz), 100, n3_f, n3_p));

    float sum_n = n + n2 + n3 * .5;
    vs_out.Normal = mat3(transpose(inverse(model))) * aNormal;
    vec4 pos4 = model * vec4(aPos, 1.0);
    vs_out.FragPos = vec3(pos4.x, sum_n, pos4.z) / pos4.w;
    gl_Position = projection * view * model * vec4(aPos.x, sum_n, aPos.z, 1.0);
    vs_out.color = (n1_g);
}
