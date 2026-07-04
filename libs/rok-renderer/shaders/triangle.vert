#version 450

layout(location = 0) out vec3 v_color;

// Baked triangle — no vertex buffer yet. Clip-space positions written with
// +Y up; the renderer's negative-viewport-height flip maps that to screen-up.
vec2 positions[3] = vec2[](
    vec2( 0.0,  0.5),   // top
    vec2(-0.5, -0.5),   // bottom-left
    vec2( 0.5, -0.5)    // bottom-right
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    v_color = colors[gl_VertexIndex];
}