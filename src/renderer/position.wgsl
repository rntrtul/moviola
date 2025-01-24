struct PositionUniform {
    rotation: vec2<f32>,
    crop_size: vec2<u32>,
    crop_start: vec2<u32>,
    translate: vec2<u32>,
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

fn rotate_90(p: vec2<f32>) -> vec2<f32> {
    return mat2x2(0, -1, 1, 0) * p;
}

fn rotate(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let cos = cos(angle);
    let sin = sin(angle);
    let r = mat2x2(cos, -sin, sin, cos);

    return (r * p);
}

@compute
@workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let input_dimensions = textureDimensions(texture);
    let output_dimensions = vec2<u32>(size.width, size.height);

    let f_output_dimensions = vec2<f32>(output_dimensions);
    let f_input_dimensions = vec2<f32>(input_dimensions);
    let output_coords = global_invocation_id.xy;

    var scale = f_input_dimensions / f_output_dimensions;
    let center = f_input_dimensions / 2.0;
    var input_pos = vec2<f32>(output_coords);

    if (position.orientation == 90.0) || (position.orientation == 270.0) {
        let out_center = f_output_dimensions / 2.0;
        input_pos = rotate_90(input_pos - out_center) + out_center.yx;
        scale = f_input_dimensions / f_output_dimensions.yx;
    }
    input_pos = input_pos * scale;

    if position.orientation != 0.0 {
//        input_pos = rotate_90(input_pos - center) + center;
    }

    if (position.mirrored == 1) {
        input_pos = vec2<f32>(abs(input_pos.x - f_output_dimensions.x), input_pos.y);
    }

    let rotation = position.rotation;
//    pos = pos - center;
//    pos = vec2<f32>((pos.x * rotation.x - pos.y * rotation.y), (pos.x * rotation.y + pos.y * rotation.x));
//    pos = pos + center;

    let output_in_bounds: bool = all(output_coords < output_dimensions);
    let valid_input_coord: bool = all(vec2f(0,0) <= input_pos) && all(input_pos < f_input_dimensions);

    if output_in_bounds && valid_input_coord {
        let uv = (input_pos + 0.5) / f_input_dimensions;
        let colour = textureSampleLevel(texture, s_texture, uv, 0.0);
        let index = (output_coords.y * size.width) + output_coords.x;
//        output[index] = pack4x8unorm(vec4f(uv, 0.0, 1.0));
        output[index] = pack4x8unorm(colour);
    }
}


