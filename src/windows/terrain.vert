#version 140
in vec3 position;
out vec3 vertex_pos;
out vec3 vertex_normal;

uniform mat4 view_matrix;

void main() {
    vertex_pos = position;
    gl_Position = view_matrix * vec4(position, 1.0);
}
