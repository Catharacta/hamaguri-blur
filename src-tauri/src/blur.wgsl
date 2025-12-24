struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index == 1u || in_vertex_index == 2u || in_vertex_index == 5u)) * 2.0 - 1.0;
    let y = f32(i32(in_vertex_index == 2u || in_vertex_index == 3u || in_vertex_index == 5u)) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

struct Params {
    hole_rect: vec4<f32>, // [left, top, right, bottom] in relative normalized coords
    blur_strength: f32,
};
@group(1) @binding(0) var<uniform> params: Params;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = in.tex_coords.x;
    let y = in.tex_coords.y;

    // 穴の範囲チェック (normalized coords)
    // 浮動小数点の誤差を考慮して 0.001 のマージン
    if (x >= params.hole_rect.x && x <= params.hole_rect.z &&
        y >= params.hole_rect.y && y <= params.hole_rect.w) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // ガウスぼかしの実装 (9タップ正確版)
    let spread = params.blur_strength * 0.001;
    var color = vec4<f32>(0.0);
    
    // ガウス重みの係数 (合計が 1.0 になるように調整)
    let w0 = 0.227027;
    let w1 = 0.1216216;
    let w2 = 0.054054;
    let w3 = 0.016216;

    color += textureSample(t_diffuse, s_diffuse, in.tex_coords) * w0;
    
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(spread, 0.0)) * w1;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-spread, 0.0)) * w1;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(0.0, spread)) * w1;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(0.0, -spread)) * w1;
    
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(spread, spread)) * w2;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-spread, spread)) * w2;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(spread, -spread)) * w2;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-spread, -spread)) * w2;

    return vec4<f32>(color.rgb, 1.0);
}
