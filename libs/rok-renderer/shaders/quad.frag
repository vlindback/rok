#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_normal;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D tex;
layout(set = 0, binding = 1) uniform Light {
    vec3 direction;   // direction the light travels
    vec3 color;
} light;

void main() {
    vec3 n = normalize(v_normal);
    // Surface-to-light is the negated travel direction.
    vec3 l = normalize(-light.direction);
    float diffuse = max(dot(n, l), 0.0);

    // Small ambient floor so faces facing away aren't pure black — cheap
    // stand-in for bounced light until real ambient/GI exists.
    float ambient = 0.1;

    vec3 albedo = texture(tex, v_uv).rgb;
    vec3 lit = albedo * light.color * (diffuse + ambient);

    out_color = vec4(lit, 1.0);
}