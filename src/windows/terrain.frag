#version 140
in vec3 vertex_pos;

out vec4 color;

uniform sampler2D land_texture;
uniform sampler2D water_texture;
uniform vec3 cam;

float tex_scale = 4000.0f;
float max_alt1 = 75.0f;
float max_alt2 = 500.0f;
vec4 sea = vec4(0.0, 0.25, 0.75, 1.0);
vec4 beach = vec4(0.75, 0.5, 0.0, 1.0);
vec4 grass = vec4(0.0, 0.8, 0.0, 1.0);
vec4 mountain = vec4(0.8, 0.0, 0.0, 1.0);

vec4 sample(sampler2D tex) {
    return texture(tex, vec2(vertex_pos.x / tex_scale, vertex_pos.z / tex_scale));
}

void main() {
    if (vertex_pos.y < 0.25) {
        color = sample(water_texture) * sea;
    } else if (vertex_pos.y < max_alt1) {
        color = sample(land_texture) * mix(beach, grass, vertex_pos.y / max_alt1);
    } else if (vertex_pos.y < max_alt1 + max_alt2) {
        color = sample(land_texture) * mix(grass, mountain, (vertex_pos.y - max_alt1) / max_alt2);
    } else {
        color = sample(land_texture) * mountain;
    }
    color = clamp(color, vec4(0.01, 0.01, 0.01, 1.0), vec4(1.0));
}
