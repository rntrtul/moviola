struct PositionUniform {
    translate: vec2i,
    scale: f32,
    rotation: f32,
    orientation: f32,
    mirrored: u32,
}

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var s_texture: sampler;
@group(0) @binding(2) var<uniform> position: PositionUniform;
@group(0) @binding(3) var output: texture_storage_2d<rgba8unorm, write>;

const rotate_90: mat2x2f = mat2x2(0, -1, 1, 0);
const rotate_180: mat2x2f = mat2x2(-1, 0, 0, -1);
const rotate_270: mat2x2f = mat2x2(0, 1, -1, 0);

fn rotate(p: vec2f, angle: f32) -> vec2f {
    let cos = cos(angle);
    let sin = sin(angle);
    let r = mat2x2(cos, -sin, sin, cos);

    return (r * p);
}

const tile_width = 8u;
const wg_x = 16u;
const wg_y = 16u;
const wg_z = 1u;
const workgroup_size = vec3u(wg_x, wg_y, wg_z);

// based on: https://github.com/LouisBavoil/ThreadGroupIDSwizzling/blob/master/ThreadGroupTilingX.hlsl
fn id_to_coord(
        local_id: vec3<u32>,
        workgroup_id: vec3<u32>,
        dispatch_size: vec3<u32>,
    ) -> vec2u {
    let ids_in_perfect_tile = tile_width * dispatch_size.y;
    let number_of_perfect_tiles = dispatch_size.x / tile_width;
    let ids_in_all_perfect_tiles = number_of_perfect_tiles *  tile_width * dispatch_size.y;
    let flat_workgroup_id = (dispatch_size.x * workgroup_id.y) + workgroup_id.x;

    let tile_id = flat_workgroup_id / ids_in_perfect_tile;
    let tile_cell = flat_workgroup_id % ids_in_perfect_tile;

    var tile_y = tile_cell / tile_width;
    var tile_x = tile_cell % tile_width;
    if ids_in_all_perfect_tiles <= flat_workgroup_id {
        let last_tile_width = dispatch_size.x % tile_width;
        tile_y = tile_cell / last_tile_width;
        tile_x = tile_cell % last_tile_width;
    }

    let swizzled_flat_workgroup_id = (tile_id * tile_width) + (tile_y * dispatch_size.x) + tile_x;

    let swizzled_workgroup_id = vec2u(
        swizzled_flat_workgroup_id % dispatch_size.x,
        swizzled_flat_workgroup_id / dispatch_size.x,
    );

    let swizzled_id = (workgroup_size.xy * swizzled_workgroup_id) + local_id.xy;

    return swizzled_id;
}

@compute
@workgroup_size(wg_x, wg_y, wg_z)
fn main(
        @builtin(local_invocation_id) local_id: vec3<u32>,
        @builtin(workgroup_id) workgroup_id: vec3<u32>,
        @builtin(num_workgroups) dispatch_size: vec3<u32>
    ) {
    let f_tex_dimensions = vec2f(textureDimensions(frame));
    let f_output_dimensions = vec2f(textureDimensions(output));
    let output_coords = id_to_coord(local_id, workgroup_id, dispatch_size);
    var tex_coords = vec2f(output_coords);

    if !all(tex_coords < f_output_dimensions) {
        return;
    }

    if position.orientation != 0.0 {
        let center = f_output_dimensions / 2.0;

        switch (u32(position.orientation)) {
            case 90u: {
                tex_coords = (rotate_90 * (tex_coords - center)) + center.yx;
            }
            case 180u: {
                tex_coords = (rotate_180 * (tex_coords - center)) + center;
            }
            case 270u: {
                tex_coords = (rotate_270 * (tex_coords - center)) + center.yx;
            }
            default: {
                tex_coords = tex_coords;
            }
        }
    }

    tex_coords = tex_coords * position.scale;

    if (position.mirrored == 1) {
        tex_coords = vec2f(abs(tex_coords.x - f_tex_dimensions.x), tex_coords.y);
    }

    if position.rotation != 0.0 {
        let tex_center = f_tex_dimensions / 2.0;
        tex_coords = rotate(tex_coords - tex_center, position.rotation) + tex_center;
    }
    
    tex_coords = tex_coords - vec2f(position.translate);

    if all(vec2f(0,0) <= tex_coords) && all(tex_coords < f_tex_dimensions) {
        let uv = (tex_coords + 0.5) / f_tex_dimensions;
        let colour = textureSampleLevel(frame, s_texture, uv, 0.0);
        textureStore(output, output_coords, colour);
    }
}
