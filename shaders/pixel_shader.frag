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

#define resolution vec2(1778, 1000)
#define PI 3.14159265358979393
//const vec3[] colors =  { vec3(0, 0.7, 0.6), vec3(0, 0, 0), vec3(0.7, 0.6, 0.0) };

float rand(vec2 c){
    return fract(sin(dot(c.xy, vec2(12.9898, 78.233))) * 43758.5453);
}

float noise(vec2 p, float freq){
    float unit = resolution.x/freq;
    vec2 ij = floor(p/unit);
    vec2 xy = mod(p, unit)/unit;
    //xy = 3.*xy*xy-2.*xy*xy*xy;
    xy = .5*(1.-cos(PI*xy));
    float a = rand((ij+vec2(0., 0.)));
    float b = rand((ij+vec2(1., 0.)));
    float c = rand((ij+vec2(0., 1.)));
    float d = rand((ij+vec2(1., 1.)));
    float x1 = mix(a, b, xy.x);
    float x2 = mix(c, d, xy.x);
    return mix(x1, x2, xy.y);
}

float pNoise(vec2 p, int res){
    float persistance = .5;
    float n = 0.;
    float normK = 0.;
    float f = 4.;
    float amp = 1.;
    int iCount = 0;
    for (int i = 0; i<50; i++){
        n+=amp*noise(p, f);
        f*=2.;
        normK+=amp;
        amp*=persistance;
        if (iCount == res) break;
        iCount++;
    }
    float nf = n/normK;
    return nf*nf*nf*nf;
}


Cell cell(int id) {
    float x = id % 11 *.1;
    float y = (float(int(id/11)) * .2) + .1;
    float xnoise =  (pNoise(vec2(x + fs_in.Time, y + fs_in.Time) * 100, 2) * 3 - .5) * .1;
    float ynoise =  (pNoise(vec2(x - 100 + fs_in.Time, y + fs_in.Time) * 100, 2) * 3- .5) * .1;
    vec2 pos = vec2(x + xnoise, y + ynoise) - vec2(-0.01, -2*0.01);
    ivec2 quad = ivec2(int(pos.x * 10), int(pos.y * 10));
    return Cell(quad, pos);
}

bool line(vec2 v1, vec2 v2, vec2 f){
    float linewidth = 0.003;
    if (min(v1.x, v2.x) > f.x || f.x > max(v1.x, v2.x)) {
        return false;
    }

    float yv = v1.y + (v2.y - v1.y)/(v2.x - v1.x) * (f.x - v1.x);
    if (f.y > yv - linewidth && f.y < yv + linewidth) {
        return true;
    }

    return false;
}

void main() {
    //    FragColor = vec;
    vec4 n = (0.0001 * vec4((fs_in.FragPos/fs_in.FragPos), 1));
    vec2 fc = (gl_FragCoord.xy) / resolution + vec2(0, 0.0);
    int c = 1;


    float epsilon = 0.01;
    Cell cells[100];


    for (int i = 0; i<55; i++) {
        Cell b = cell(i);
        cells[i] = b;
        if (distance(b.pos, fc) < epsilon) {
            c = 2;
        }
    }
/*
    for (int i = 0; i < 10; i++) {
        for (int iy = 0; iy < 10; iy++) {
            Cell thisCell = cells[i][iy];
            for (int ox = -1; ox < 2; ox++) {
                for (int oy = -1; oy < 2; oy++) {
                    if (ox + i > 9 || oy + iy > 9 || ox + i < 0 || oy + iy < 0 || (ox == 0 && oy == 0)){
                        continue;
                    }
                    Cell otherCell = cells[ox + i][oy + iy];
                    //            if (!(otherCell.box.y == thisCell.box.y && otherCell.box.x == thisCell.box.x)) {
                    //                continue;
                    //            }

                    if (distance(otherCell.pos, thisCell.pos) > 0.1) {
                        continue;
                    }

                    if (line(thisCell.pos, otherCell.pos, fc)){
                        c = 0;
                        //            tv = true;
                    }
                }
            }
        }
    }
*/
    if (c == 0) {
        FragColor = n + vec4(0, 0.0, 0.0, 1.0);
    }
    else if (c == 1) {
        FragColor = n + vec4(0, 0.7, 0.6, 1.0);
    }
    else if (c == 2) {
        FragColor = n + vec4(0.7, 0.6, 0.0, 1.0);
    }
}
