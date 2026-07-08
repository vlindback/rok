#version 450
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec3 in_normal;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec3 v_normal;

layout(push_constant) uniform Push {
    mat4 mvp;
    mat4 model;
} pc;

void main() {
    gl_Position = pc.mvp * vec4(in_position, 1.0);
    v_uv = in_uv;
    // World-space normal. mat3(model) is correct for rotation + uniform scale;
    // non-uniform scale needs the inverse-transpose (normal matrix) — deferred
    // until a non-uniformly-scaled object actually appears.
    v_normal = mat3(pc.model) * in_normal;
}