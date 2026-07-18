#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec3 v_world_pos;
layout(location = 3) in vec3 v_tangent;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D albedo_tex;
layout(set = 0, binding = 1) uniform sampler2D normal_tex;
layout(set = 0, binding = 2) uniform sampler2D roughness_tex;
// binding 2 (roughness) still unsampled — Phase D

const int LIGHT_DIRECTIONAL = 0;
const int LIGHT_POINT = 1;
const int MAX_LIGHTS = 16;

struct Light {
    vec3 direction;
    uint kind;
    vec3 color;
    vec3 position;
};

layout(set = 1, binding = 0) uniform Frame {
    mat4 view_proj;
    vec4 camera_pos;
} frame;

layout(set = 1, binding = 1) uniform Lights {
    uint count;
    Light lights[MAX_LIGHTS];
} lights;

void main() {
    // --- Build the TBN: tangent space -> world space ---------------------
    // Gram-Schmidt re-orthogonalize the tangent against the normal. After
    // interpolation across the face, T and N drift slightly out of square;
    // this snaps T back to perpendicular. Cheap, and it matters on real meshes.
    vec3 N = normalize(v_normal);
    vec3 T = normalize(v_tangent - N * dot(N, v_tangent));
    vec3 B = cross(N, T);            // bitangent derived, not stored
    mat3 TBN = mat3(T, B, N);        // columns: maps tangent-space -> world

    // --- Sample the normal map -------------------------------------------
    // Stored as [0,1] RGB; unpack to [-1,1] direction. This is why the slot is
    // UNORM, not SRGB — it's a vector, not a color. sRGB-decoding it here would
    // silently skew every normal.
    vec3 tangent_normal = texture(normal_tex, v_uv).rgb * 2.0 - 1.0;

    // If bricks look CAVED IN instead of raised, this map is DirectX-convention
    // and needs: tangent_normal.g = -tangent_normal.g;
    // (OpenGL/glTF convention = +G is up, which is what we assume.)

    vec3 n = normalize(TBN * tangent_normal);   // <-- per-pixel world normal

    // --- Lighting: identical to before, but `n` is now per-PIXEL ----------
    vec3 albedo = texture(albedo_tex, v_uv).rgb;
    vec3 view_dir = normalize(frame.camera_pos.xyz - v_world_pos);

    // Map roughness -> Blinn-Phong shininess. This is an APPROXIMATION - a
    // real PBR BRDF (GGX) takes roughness directly. Rough surfaces scatter
    // light broadly (low exponent), smooth ones focus it (high exponent).
    // The exponential form gives a perceptually reasonable curve.
    float roughness = texture(roughness_tex, v_uv).r;
    float shininess = pow(2.0, 10.0 * (1.0 - roughness) + 1.0);

    // Rough surfaces also reflect LESS energy in the specular lobe — a crude
    // stand-in for energy conservation until a real BRDF handles it properly.
    float spec_strength = 1.0 - roughness;

    vec3 diffuse_accum = vec3(0.0);
    vec3 specular_accum = vec3(0.0);

    for (uint i = 0u; i < lights.count; i++) {
        Light light = lights.lights[i];
        vec3 l;
        float attenuation = 1.0;

        if (light.kind == uint(LIGHT_POINT)) {
            vec3 to_light = light.position - v_world_pos;
            float dist = length(to_light);
            l = to_light / dist;
            attenuation = 1.0 / (dist * dist);
        } else {
            l = normalize(-light.direction);
        }

        float diff = max(dot(n, l), 0.0);
        diffuse_accum += light.color * diff * attenuation;

        if (diff > 0.0) {
            vec3 halfway = normalize(l + view_dir);
            float spec = pow(max(dot(n, halfway), 0.0), shininess);
            specular_accum += light.color * spec * spec_strength * attenuation;
        }
    }

    float ambient = 0.1;
    vec3 lit = albedo * (diffuse_accum + ambient) + specular_accum;
    out_color = vec4(lit, 1.0);
}