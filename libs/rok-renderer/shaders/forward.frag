#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec3 v_world_pos;
layout(location = 3) in vec3 v_tangent;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D albedo_tex;
layout(set = 0, binding = 1) uniform sampler2D normal_tex;
layout(set = 0, binding = 2) uniform sampler2D metallic_roughness_tex ;
layout(set = 0, binding = 3) uniform sampler2D emissive_tex;
layout(set = 0, binding = 4) uniform MaterialFactors {
    vec4  base_color_factor;   // offset 0   (16 bytes)
    vec4  emissive_factor;     // offset 16  (rgb + unused .w)
    float metallic_factor;     // offset 32
    float roughness_factor;    // offset 36
    float normal_scale;        // offset 40
    float occlusion_strength;  // offset 44
} material;                    // total: 48 bytes (already a multiple of 16)

const float PI = 3.14159265359;

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

// GGX / Trowbridge-Reitz normal distribution.
// N·H = how aligned the surface is with the halfway vector; a = roughness².
float distribution_ggx(float NdotH, float roughness) {
    float a = roughness * roughness;       // perceptual→physical: artists author
    float a2 = a * a;                      // roughness "linear", the BRDF wants a²
    float d = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

// Schlick-GGX: one direction's geometry factor. NdotX is N·L or N·V.
float geometry_schlick_ggx(float NdotX, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;                    // direct-lighting remap of roughness
    return NdotX / (NdotX * (1.0 - k) + k);
}

// Smith: combine shadowing (light side) and masking (view side).
float geometry_smith(float NdotV, float NdotL, float roughness) {
    return geometry_schlick_ggx(NdotV, roughness)   // masking (view)
         * geometry_schlick_ggx(NdotL, roughness);  // shadowing (light)
}

// Fresnel-Schlick: reflectance rising from F0 (head-on) to 1.0 (grazing).
// cos_theta is typically V·H (view vs halfway).
vec3 fresnel_schlick(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

void main() {
    // --- Build the TBN: tangent space -> world space ---------------------
    // Gram-Schmidt re-orthogonalize the tangent against the normal. After
    // interpolation across the face, T and N drift slightly out of square;
    // this snaps T back to perpendicular. Cheap, and it matters on real meshes.
    vec3 N = normalize(v_normal);
    vec3 n;
    if (dot(v_tangent, v_tangent) > 1e-6) {
        // Has a tangent basis → normal mapping
        vec3 T = normalize(v_tangent - N * dot(N, v_tangent));
        vec3 B = cross(N, T);
        mat3 TBN = mat3(T, B, N);
        vec3 tn = texture(normal_tex, v_uv).rgb * 2.0 - 1.0;
        n = normalize(TBN * tn);
    } else {
        n = N;   // no tangent → geometry normal, normal map skipped
    }

    vec3 albedo = texture(albedo_tex, v_uv).rgb * material.base_color_factor.rgb;
    vec3 view_dir = normalize(frame.camera_pos.xyz - v_world_pos);

    vec4 mr = texture(metallic_roughness_tex, v_uv);
    float roughness = mr.g * material.roughness_factor;
    float metallic  = mr.b * material.metallic_factor;

    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    float NdotV = max(dot(n, view_dir), 0.0);

    vec3 Lo = vec3(0.0);   // accumulated outgoing radiance

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

        vec3  h     = normalize(l + view_dir);
        float NdotL = max(dot(n, l), 0.0);
        float NdotH = max(dot(n, h), 0.0);
        float VdotH = max(dot(view_dir, h), 0.0);

        vec3 radiance = light.color * attenuation;

        // Cook-Torrance specular = D * G * F / (4 * NdotV * NdotL)
        float D = distribution_ggx(NdotH, roughness);
        float G = geometry_smith(NdotV, NdotL, roughness);
        vec3  F = fresnel_schlick(VdotH, F0);

        vec3 numerator   = D * G * F;
        float denominator = 4.0 * NdotV * NdotL + 0.0001;   // +epsilon: never divide by 0
        vec3 specular    = numerator / denominator;

        // Energy conservation: F is the fraction reflected (specular);
        // (1 - F) is what's left for diffuse. Metals kill diffuse entirely.
        vec3 kD = (vec3(1.0) - F) * (1.0 - metallic);
        vec3 diffuse = kD * albedo / PI;

        Lo += (diffuse + specular) * radiance * NdotL;
    }

    vec3 ambient = vec3(0.03) * albedo;   // flat fill so shadowed areas aren't pure black
    vec3 color = ambient + Lo;
    vec3 emissive = texture(emissive_tex, v_uv).rgb * material.emissive_factor.rgb;
    color += emissive;

    out_color = vec4(color, 1.0);
}