// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);

    return out;
}

// Fragment shader
struct Parameters {
    contrast: f32,
    brightness: f32,
    saturation: f32,
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> params: Parameters;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let colour = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    return apply_colour_effects(colour);
}

fn apply_colour_effects(colour: vec4<f32> ) -> vec4<f32> {
    let contrast_bright = contrast_brigtness(colour);

    return saturate(contrast_bright);
}

fn contrast_brigtness(colour: vec4<f32>) -> vec4<f32> {
    //todo: should use midpoint as pow(0.5, 2.2)
    return ((colour - 0.5 ) * params.contrast) + 0.5 + params.brightness;
}

fn saturate(colour: vec4<f32>) -> vec4<f32> {
    let luma = dot(colour, vec4<f32>(0.216279, 0.7515122, 0.0721750, 0.0));
    return luma + params.saturation * (colour - luma);
}
