struct PositionUniform {
    rotation: vec2<f32>,
    crop_size: vec2<u32>,
    crop_start: vec2<u32>,
    translate: vec2<u32>,
}

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> position: PositionUniform;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var s_texture: sampler;

@compute
@workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let dimensions = textureDimensions(texture);
    let dest_coords = global_invocation_id.xy;

    let center = vec2<f32>(dimensions) / 2.0;
    let pos = vec2<f32>(dest_coords) - center;
    let rotation = position.rotation;

    let tex_coords =
            vec2<f32>((pos.x * rotation.x - pos.y * rotation.y), (pos.x * rotation.y + pos.y * rotation.x)) + center;

    let bounds = dimensions;
    let dest_in_bounds: bool = all(dest_coords < bounds);
    let valid_origin_coord: bool = all(vec2f(0,0) <= tex_coords) && all(tex_coords < vec2<f32>(bounds));

    if dest_in_bounds && valid_origin_coord {
        let uv = (tex_coords + 0.5) / vec2<f32>(textureDimensions(output_texture));
        let colour = textureSampleLevel(texture, s_texture, uv, 0.0);
        textureStore(output_texture, dest_coords, colour);
    }
}
