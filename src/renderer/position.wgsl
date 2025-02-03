struct PositionUniform {
    translate: vec2i,
    scale: f32,
    rotation: f32,
    orientation: f32,
    mirrored: u32,
}

struct FrameSize{
    width: u32,
    height: u32,
}

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var s_texture: sampler;
@group(0) @binding(2) var<uniform> position: PositionUniform;
@group(0) @binding(3) var<uniform> size: FrameSize;
@group(0) @binding(4) var<storage, read_write> output: array<u32>;

const rotate_90: mat2x2f = mat2x2(0, -1, 1, 0);
const rotate_180: mat2x2f = mat2x2(-1, 0, 0, -1);
const rotate_270: mat2x2f = mat2x2(0, 1, -1, 0);

fn rotate(p: vec2f, angle: f32) -> vec2f {
    let cos = cos(angle);
    let sin = sin(angle);
    let r = mat2x2(cos, -sin, sin, cos);

    return (r * p);
}

@compute
@workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let tex_dimensions = textureDimensions(texture);
    let output_dimensions = vec2u(size.width, size.height);

    let f_tex_dimensions = vec2f(tex_dimensions);
    let f_output_dimensions = vec2f(output_dimensions);
    var tex_coords = vec2f(global_invocation_id.xy);

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
        let colour = textureSampleLevel(texture, s_texture, uv, 0.0);
        let index = (global_invocation_id.y * size.width) + global_invocation_id.x;
        output[index] = pack4x8unorm(colour);
    }
}
