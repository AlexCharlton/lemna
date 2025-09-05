#version 450

layout(set=0, binding = 0)
uniform Globals {
  mat4 viewport;
};

layout(location = 0) in vec2 v_Pos;
layout(location = 1) in vec2 v_TexPos;

layout(location = 2) in vec3 i_Pos;

layout(location = 0) out vec2 f_TexPos;

void main() {
  gl_Position = viewport * vec4(vec3(v_Pos + round(i_Pos.xy),  i_Pos.z), 1.0);
  f_TexPos = v_TexPos;
}
