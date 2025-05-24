#version 460 core
//processed
//T: LOCATIONS

//T: STD140

//T: UNIFORMS

//T: OUT
uniform float time;
void main()
{

    //T: PASSTHROUGHS
    //L: IF NORMAL
    vs_out.Normal = mat3(transpose(inverse(model))) * aNormal;
    //L: ENDIF
    vs_out.FragPos = vec3(model * vec4(aPos, 1.0));
    vs_out.Time = time;
    gl_Position = projection * view * model * vec4(aPos.xyz, 1.0);

}
