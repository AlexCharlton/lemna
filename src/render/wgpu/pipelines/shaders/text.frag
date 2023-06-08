#version 450

layout(location = 0) in vec2 v_TexPos;
layout(location = 1) in vec4 v_Color;

layout(location = 0) out vec4 f_Color;

layout(set = 1, binding = 0) uniform texture2D t_1D;
layout(set = 1, binding = 1) uniform sampler s_text;

void main() {
  float alpha = texture(sampler2D(t_1D, s_text), v_TexPos).r;
  if (alpha <= 0.0) {
    discard;
    // f_Color = vec4(1.0, 0.0, 1.0, 1.0);
  } else {
    f_Color = v_Color * vec4(1.0, 1.0, 1.0, alpha);
  }

}
