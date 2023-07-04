#version 450

layout(location = 0) in vec2 v_TexPos;

layout(location = 0) out vec4 f_Color;

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler samp;

void main() {
  vec4 value = texture(sampler2D(tex, samp), v_TexPos);
  if (value.a <= 0.0) {
    discard;
    // f_Color = vec4(1.0, 0.0, 1.0, 1.0);
  } else {
    f_Color = value;
  }

}
