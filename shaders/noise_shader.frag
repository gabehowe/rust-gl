#version 460 core
out vec4 FragColor;

in VS_OUT {
    vec3 Normal;
    vec3 FragPos;
    float color;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

void main() {
    vec3 lightPos = vec3(1.2f, 1.0f, 4.0f);
    vec3 lightDir = normalize(lightPos - fs_in.FragPos);
    vec3 normal = normalize(cross(dFdx(fs_in.FragPos), dFdy(fs_in.FragPos)));
    float diff = max(dot(normal, normalize(lightDir)), 0.0);
    vec3 diffuse = diff * vec3(1.0f, 1.0f, 1.0f);
    vec3 ambientColor = vec3(0.0f, fs_in.color, 0.0f);
    FragColor = vec4(diffuse + ambientColor, 1.0);

}
