#version 460 core
out vec4 FragColor;

in GS_OUT {
    vec3 Normal;
    vec3 FragPos;
} fs_in;

layout (std140) uniform Matrices {
    vec3 cameraPos;
    mat4 view;
    mat4 projection;
};

void main() {
    vec3 lightPos = vec3(1.2f, -1.0f, 4.0f);
    vec3 lightDir = normalize(lightPos - fs_in.FragPos);
    float diff = max(dot(normalize(fs_in.Normal), normalize(lightDir)), 0.0);
    vec3 diffuse = diff * vec3(1.0f, 1.0f, 1.0f);
    vec3 ambientColor = vec3(1.0f, 0.5f, 0.2f);
    FragColor = vec4(diffuse + ambientColor, 1.0);
//    FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
