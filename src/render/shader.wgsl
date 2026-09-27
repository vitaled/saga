// Sprite shader used by the SAGA renderer: one instanced, textured,
// tinted quad per draw command.

struct Uniforms {
    // Physical size of the surface in pixels.
    screen: vec2<f32>,
    // Letterbox offset of the virtual viewport inside the surface.
    offset: vec2<f32>,
    // Virtual pixel -> physical pixel scale.
    scale: vec2<f32>,
    padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) corner: vec2<f32>,
};

struct InstanceInput {
    @location(1) position: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) uv_offset: vec2<f32>,
    @location(4) uv_size: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let local = vertex.corner * instance.size;
    let pixel = uniforms.offset + (instance.position + local) * uniforms.scale;
    let ndc = vec2<f32>(
        pixel.x / uniforms.screen.x * 2.0 - 1.0,
        1.0 - pixel.y / uniforms.screen.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = instance.uv_offset + vertex.corner * instance.uv_size;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
    if (sampled.a <= 0.0) {
        discard;
    }
    return sampled;
}
