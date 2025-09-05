#version 450

layout(location = 0) in vec2 v_Pos;
layout(location = 1) in vec2 v_TexPos;

layout(location = 0) out vec2 f_TexPos;

void main() {
  gl_Position = vec4(v_Pos, 0.0, 1.0);
  f_TexPos = v_TexPos;
}
