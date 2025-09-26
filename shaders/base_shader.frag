#version 460 core
//processed
layout (std140) uniform Matrices {
	vec3 cameraPos;
	mat4 view;
	mat4 projection;
};
layout (std140, binding=1) uniform World {
	vec4 ambient;
};

in VS_OUT {
	vec3 Normal;
	vec3 FragPos;
	float Time;
#ifdef TEXTURES
	vec2 TexCoord;
#endif
} fs_in;

out vec4 FragColor;
#ifdef DIFFUSE_TEXTURE
uniform sampler2D diffuse;
#else
uniform vec4 diffuse;
#endif

#ifdef EMISSIVE_TEXTURE
uniform sampler2D emissive;
#else
uniform vec4 emissive;
#endif

float specular_exponent = 256.0;

#ifdef SPECULAR_TEXTURE
uniform sampler2D specular;
#else
uniform float specular;
#endif



void main() {
#ifdef DIFFUSE_TEXTURE
	vec4 diffuse = texture(diffuse, fs_in.TexCoord);
#endif
#ifdef EMISSIVE_TEXTURE
	vec4 emissive = texture(emissive, fs_in.TexCoord);
#endif
#ifdef SPECULAR_TEXTURE
	float specular = texture(specular, fs_in.TexCoord);
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
