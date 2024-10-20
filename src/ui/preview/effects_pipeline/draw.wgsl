// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);

    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var coords = vec2<u32>(in.tex_coords * vec2<f32>(textureDimensions(t_diffuse)));
    return textureLoad(t_diffuse, coords);
}
