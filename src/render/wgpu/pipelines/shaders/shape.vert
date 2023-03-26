#version 450

layout(set=0, binding = 0)
uniform Globals {
  mat4 viewport;
};

layout(location = 0) in vec2 v_Pos;
layout(location = 1) in vec2 v_Norm;

layout(location = 2) in vec3 i_Pos;
layout(location = 3) in vec4 i_Color;
layout(location = 4) in float i_StrokeWidth;

layout(location = 0) out vec4 f_Color;

void main() {
  vec2 local_pos = v_Pos + v_Norm * i_StrokeWidth;
  gl_Position = viewport *
    vec4(
         vec3(
              (local_pos + i_Pos.xy),
              i_Pos.z),
         1.0);
  f_Color = i_Color;
}
