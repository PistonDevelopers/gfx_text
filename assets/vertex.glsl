#version 150 core

in vec2 a_Pos;
in vec4 a_Color;
in vec2 a_TexCoord;
in vec4 a_World_Pos;
in int a_Screen_Rel;
out vec4 v_Color;
out vec2 v_TexCoord;
uniform vec2 u_Screen_Size;
uniform mat4 u_Proj;

void main() {
    // On-screen offset from text origin.
    vec2 v_Screen_Offset = vec2(
        2 * a_Pos.x / u_Screen_Size.x - 1,
        1 - 2 * a_Pos.y / u_Screen_Size.y
    );
    vec4 v_Screen_Pos = u_Proj * a_World_Pos;
    vec2 v_World_Offset = a_Screen_Rel == 0
        // Perspective divide to get normalized device coords.
        ? vec2 (
            v_Screen_Pos.x / v_Screen_Pos.z + 1,
            v_Screen_Pos.y / v_Screen_Pos.z - 1
        ) : vec2(0.0, 0.0);

    v_Color = a_Color;
    v_TexCoord = a_TexCoord;
    gl_Position = vec4(v_World_Offset + v_Screen_Offset, 0.0, 1.0);
}
