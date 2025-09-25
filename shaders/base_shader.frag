#version 460 core
//processed
// STD140s
layout (std140) uniform Matrices {
	vec3 cameraPos;
	mat4 view;
	mat4 projection;
};
layout (std140, binding=1) uniform World {
	vec4 ambient;
};

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
#ifdef TEXTURES
layout (location = 2) in vec2 aTexCoord;
#endif




out vec4 FragColor;


uniform mat4 model;

//T: TEXTURES

void main() {
    //T: LOGIC
#ifdef TEXTURES
	vs_out.TexCoord = aTexCoord;
#endif
    vec3 lightPos = vec3(-10.0f, 15.0f, 1.0f);
    vec3 lightDir = normalize(lightPos - fs_in.FragPos);
    vec3 normal = normalize(cross(dFdx(fs_in.FragPos), dFdy(fs_in.FragPos)));
    float diff = max(dot(normal, normalize(lightDir)), 0.0);
    float spec = pow(max(dot(normal, normalize(lightDir)), 0.0), specular_exponent);
    FragColor = (specular * spec) + emissive + (diff * diffuse) + ambient;
//    FragColor = vec4(1.0f,1.0f,1.0f,1.0f);
    //    FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
