#version 460 core
//T: STD140

//T: IN

//T: OUT

//T: UNIFORMS

//T: TEXTURES

void main() {
    //T: LOGIC
    vec3 lightPos = vec3(1.2f, 1.0f, 1.0f);
    vec3 lightDir = normalize(lightPos - fs_in.FragPos);
    vec3 normal = normalize(cross(dFdx(fs_in.FragPos), dFdy(fs_in.FragPos)));
    float diff = max(dot(normal, normalize(lightDir)), 0.0);
    float spec = pow(max(dot(normal, normalize(lightDir)), 0.0), specular_exponent);
    FragColor = (specular * spec) + emissive + (diff * diffuse) + ambient + vec4(0.0, 0.0,0.0, 1.0);
    //    FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
