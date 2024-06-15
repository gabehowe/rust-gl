#version 460 core
//T: STD140

//T: IN

//T: OUT

//T: UNIFORMS

//T: TEXTURES

void main() {
    //T: LOGIC
    FragColor = (specular + emissive + diffuse + ambient) + vec4(0.0, 0.0,0.0, 1.0);
    //    FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}
