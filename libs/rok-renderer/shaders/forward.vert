#version 450
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec3 in_normal;
layout(location = 3) in vec3 in_tangent;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec3 v_normal;
layout(location = 2) out vec3 v_world_pos;
layout(location = 3) out vec3 v_tangent;

layout(push_constant) uniform Push {
    mat4 model;
} pc;

layout(set = 1, binding = 0) uniform Frame {
    mat4 view_proj;
    vec4 camera_pos;
} frame;

void main() {
    vec4 world = pc.model * vec4(in_position, 1.0);
    gl_Position = frame.view_proj * world;
    v_uv = in_uv;
    v_normal = mat3(pc.model) * in_normal;
    v_tangent = mat3(pc.model) * in_tangent;
    v_world_pos = world.xyz;
}