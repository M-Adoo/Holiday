#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in vec2 v_text_size;
layout(location=2) in vec2 v_text_offset;
layout(location=3) in vec2 v_atlas_size;
layout(location=4) in vec2 v_glyph_tex_pos;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 1) uniform texture2D t_atals;
layout(set = 0, binding = 2) uniform texture2D t_glyph;
layout(set = 0, binding = 3) uniform sampler s_sampler;

void main() {

    // For now, always use repeat pattern to fill.
    vec2 tex_pos = v_tex_coords - v_text_offset;
    tex_pos[0] = mod(tex_pos[0], v_text_size[0]);
    tex_pos[1] = mod(tex_pos[1], v_text_size[1]);
    tex_pos += v_text_offset;

    tex_pos[0] = tex_pos[0] / v_atlas_size[0];
    tex_pos[1] = tex_pos[1] / v_atlas_size[1];
    
    float alpha = 1.0;
    if (v_glyph_tex_pos[0] >= 0) {
        alpha = texture(sampler2D(t_glyph, s_sampler), v_glyph_tex_pos).r; 
    }
    if (alpha <= 0.0) {
        discard;
    }
    f_color = texture(sampler2D(t_atals, s_sampler), tex_pos);

    // rbga fomat texture store in a Bgra8UnormSrgb texture.
    f_color = vec4(f_color.b, f_color.g, f_color.r, f_color.a * alpha);
}