#version 460 core
out vec4 FragColor;

in VS_OUT {
    vec3 Normal;
    vec3 FragPos;
    float Time;
    vec4 bounds;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

#define resolution vec2(1778, 1000)
#define PI 3.14159265358979393
//const vec3[] colors =  { vec3(0, 0.7, 0.6), vec3(0, 0, 0), vec3(0.7, 0.6, 0.0) };
vec3 rgb2hsv(vec3 c)
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}
// Minimum temperature to represent.
float minTemp = 1500.;

// Maximum temperature to represent.
float maxTemp = 15000.;

float calcRed(float temp) {

    float red;

    if ( temp <= 6600. ) {

        red = 1.;
    }
    else {
        temp = temp - 6000.;

        temp = temp / 100.;

        red = 1.29293618606274509804 * pow(temp, -0.1332047592);

        if (red < 0.) {
            red = 0.;
        }
        else if (red > 1.) {
            red = 1.;
        }
    }

    return red;
}

float calcGreen(float temp) {

    float green;

    if ( temp <= 6600. ) {
        temp = temp / 100.;

        green = 0.39008157876901960784 * log(temp) - 0.63184144378862745098;

        if (green < 0.) {
            green = 0.;
        }
        else if (green > 1.) {
            green = 1.;
        }
    }
    else {
        temp = temp - 6000.;

        temp = temp / 100.;

        green = 1.12989086089529411765 * pow(temp, -0.0755148492);

        if (green < 0.) {
            green = 0.;
        }
        else if (green > 1.) {
            green = 1.;
        }
    }

    return green;
}

float calcBlue(float temp) {

    float blue;

    if ( temp <= 1900. ) {
        blue = 0.;
    }
    else if ( temp >= 6600.) {
        blue = 1.;
    }
    else {
        temp = temp / 100.;

        blue = .00590528345530083 * pow(temp, 1.349167257362226); // R^2 of power curve fit: 0.9996
        blue = 0.54320678911019607843 * log(temp - 10.0) - 1.19625408914;

        if (blue < 0.) {
            blue = 0.;
        }
        else if (blue > 1.) {
            blue = 1.;
        }
    }

    return blue;
}

void main() {
    vec2 fc = (fs_in.FragPos.xz) + vec2(-0.5, -0.5);
//    float xs = (-3.25 - 0.83) * -fc.x + (-3.25 + 0.83) / 4;
//    float ys = (-1.12 - 1.12) * fc.y;
    float fac = 1;
    float xs = (((fs_in.bounds[0] - fs_in.bounds[1]) * -fc.x + (fs_in.bounds[0] + fs_in.bounds[1]) / 4) * fac);
    float ys = ( fac*((fs_in.bounds[2] - fs_in.bounds[3]) * fc.y + (fs_in.bounds[2] + fs_in.bounds[3]) / 2) );
    int iteration = 0;
    float nx = 0;
    float ny = 0;
    float x2 = 0;
    float y2 = 0;
    float w = 0;
    bool is_too_high = false;
    while (x2 + y2 <= 4) {
        ny = (nx + nx)*ny + ys;
        nx = x2 - y2 + xs;
        x2 = nx * nx;
        y2 = ny * ny;
        //w = (nx+ny) * (nx+ny);
        iteration += 1;
        if (iteration >= 1000) {
            is_too_high = true;
            break;
        }
    }

    if (is_too_high) {
        FragColor= vec4(1,0,0,0);
    }
    else {

        float temp = minTemp + float(iteration / 250) * maxTemp;
        vec3 color = rgb2hsv(vec3(calcRed(temp), calcGreen(temp), calcBlue(temp)));
        color.z = float(iteration)/90.0;
        FragColor = vec4(hsv2rgb(color),1.0);
    }
}
