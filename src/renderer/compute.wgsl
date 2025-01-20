const chunk_width = 256;

@group(0) @binding(0) var padded_texture: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var<storage, read_write> unpadded_buffer: array<u32>;

const crop_start = vec2<u32>(0, 0);
const crop_size = vec2<u32>(0, 0);
const rotation = vec2<f32>(1.0, 0.0);

fn between_vecs(val: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return (low.x <= val.x) && (val.x < high.x) && (low.y <= val.y) && (val.y < high.y);
}

fn fully_strictly_less(a: vec2<f32>, b: vec2<f32>) -> bool {
    let result = a < b;
    return result.x && result.y;
}

@compute
@workgroup_size(chunk_width, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let dimensions = textureDimensions(padded_texture);
    let dest_coords = global_invocation_id.xy + crop_start;

    let center = vec2<f32>(dimensions) / 2.0;
    let pos = vec2<f32>(dest_coords) - center;

    let tex_coords =
        vec2<f32>((pos.x * rotation.x - pos.y * rotation.y), (pos.x * rotation.y + pos.y * rotation.x)) + center;

    let bounds = vec2<f32>(dimensions - crop_size);

    if all(fully_strictly_less(vec2<f32>(dest_coords), bounds) && between_vecs(tex_coords, vec2f(0,0), bounds)){
        let index = dest_coords.x + (dest_coords.y * dimensions.x);
        unpadded_buffer[index] = pack4x8unorm(textureLoad(padded_texture, vec2<u32>(tex_coords)));
    }
}
