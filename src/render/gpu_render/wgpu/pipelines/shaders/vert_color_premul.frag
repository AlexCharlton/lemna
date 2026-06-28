#version 450

layout(location = 0) in vec4 v_Color;
layout(location = 0) out vec4 f_Color;

void main() {
    f_Color = vec4(v_Color.rgb * v_Color.a, v_Color.a);
}
