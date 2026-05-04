#version 150 core

in vec4 v_Color;
in vec2 v_TexCoord;
out vec4 o_Color;
uniform sampler2D t_Color;

void main() {
    vec4 t_Font_Color = texture(t_Color, v_TexCoord);
    o_Color = vec4(v_Color.rgb, t_Font_Color.r * v_Color.a);
}
