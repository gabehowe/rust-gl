#version 460 core
//processed
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
#ifdef TEXTURES
layout (location = 2) in vec2 aTexCoord;
#endif

layout (std140) uniform Matrices {
	vec3 cameraPos;
	mat4 view;
	mat4 projection;
};
layout (std140, binding=1) uniform World {
	vec4 ambient;
};

uniform mat4 model;

out VS_OUT {
	vec3 Normal;
	vec3 FragPos;
	float Time;
#ifdef TEXTURES
	vec2 TexCoord;
#endif
} vs_out;

uniform float time;

void main()
{
#ifdef TEXTURES
	 vs_out.TexCoord = aTexCoord;
#endif
    vs_out.Normal = mat3(transpose(inverse(model))) * aNormal;
    vs_out.FragPos = vec3(model * vec4(aPos, 1.0));
    vs_out.Time = time;
    gl_Position = projection * view * model * vec4(aPos.xyz, 1.0);

}
