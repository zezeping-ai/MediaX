struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0)
  );
  var uvs = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(2.0, 1.0),
    vec2<f32>(0.0, -1.0)
  );
  var out: VsOut;
  out.pos = vec4<f32>(positions[idx], 0.0, 1.0);
  out.uv = uvs[idx];
  return out;
}

@group(0) @binding(0) var tex_y: texture_2d<f32>;
@group(0) @binding(1) var tex_u: texture_2d<f32>;
@group(0) @binding(2) var tex_v: texture_2d<f32>;
@group(0) @binding(3) var tex_uv: texture_2d<f32>;
@group(0) @binding(4) var tex_y_16: texture_2d<f32>;
@group(0) @binding(5) var tex_uv_16: texture_2d<f32>;
@group(0) @binding(6) var samp: sampler;
struct ColorParams {
  y_offset: f32,
  y_scale: f32,
  uv_offset: f32,
  uv_scale: f32,
  row0: vec4<f32>,
  row1: vec4<f32>,
  row2: vec4<f32>,
};
@group(0) @binding(7) var<uniform> color_params: ColorParams;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let layout_mode = color_params.row0.w;
  var y: f32;
  var uv: vec2<f32>;
  if (layout_mode >= 1.5) {
    y = (textureSample(tex_y_16, samp, in.uv).r - color_params.y_offset) * color_params.y_scale;
    uv = textureSample(tex_uv_16, samp, in.uv).rg;
  } else if (layout_mode >= 0.5) {
    y = (textureSample(tex_y, samp, in.uv).r - color_params.y_offset) * color_params.y_scale;
    uv = textureSample(tex_uv, samp, in.uv).rg;
  } else {
    y = (textureSample(tex_y, samp, in.uv).r - color_params.y_offset) * color_params.y_scale;
    uv = vec2<f32>(
      textureSample(tex_u, samp, in.uv).r,
      textureSample(tex_v, samp, in.uv).r
    );
  }
  let u = (uv.x - color_params.uv_offset) * color_params.uv_scale;
  let v = (uv.y - color_params.uv_offset) * color_params.uv_scale;
  let yuv = vec3<f32>(y, u, v);
  let r = dot(color_params.row0.xyz, yuv);
  let g = dot(color_params.row1.xyz, yuv);
  let b = dot(color_params.row2.xyz, yuv);
  return vec4<f32>(r, g, b, 1.0);
}
