#version 450

layout(location = 0) in vec2 v_TexPos;

layout(location = 0) out vec4 f_Color;

layout(set = 0, binding = 0) uniform texture2D t_1D;
layout(set = 0, binding = 1) uniform sampler s_msaa;

void main() {
  vec4 c = texture(sampler2D(t_1D, s_msaa), v_TexPos);
  if (c.a <= 0.0) {
    discard;
  }

  f_Color = c;
}
