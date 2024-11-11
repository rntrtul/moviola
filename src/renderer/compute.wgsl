const chunk_width = 256;

@group(0) @binding(0) var padded_texture: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var<storage, read_write> unpadded_buffer: array<u32>;

@compute
@workgroup_size(chunk_width, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let dimensions = textureDimensions(padded_texture);
    let coords = global_invocation_id.xy;

    if (all(coords < dimensions)) {
        let index = coords.y * dimensions.x + coords.x;
        unpadded_buffer[index] = pack4x8unorm(textureLoad(padded_texture, coords));
    }
}
