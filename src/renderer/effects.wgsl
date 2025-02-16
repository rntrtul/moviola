struct Parameters {
    contrast: f32,
    brightness: f32,
    saturation: f32,
}

@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var<uniform> params: Parameters;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let coords = global_invocation_id.xy;

    if all(coords < textureDimensions(output)) {
        let colour = textureLoad(input, coords);
        let final_colour = apply_colour_effects(colour);

        textureStore(output, coords, final_colour);
    }
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