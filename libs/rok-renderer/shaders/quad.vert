#version 450
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_color;
layout(location = 2) in vec2 in_uv;

layout(location = 0) out vec3 v_color;
layout(location = 1) out vec2 v_uv;

layout(push_constant) uniform Push { mat4 mvp; } pc;

void main() {
    gl_Position = pc.mvp * vec4(in_position, 1.0);
    v_color = in_color;
    v_uv = in_uv;
}