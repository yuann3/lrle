// Solid Shaded Terrain Shader
//
// Renders terrain with directional lighting for a 3D shaded appearance.
// Supports both the terrain vertex color and lighting calculations.

// ============================================================================
// Uniforms
// ============================================================================

/// Camera and lighting uniforms.
struct Uniforms {
    /// Combined view * projection matrix for transforming world -> clip space
    view_proj: mat4x4<f32>,
    /// Light direction (normalized, pointing toward light source)
    light_dir: vec3<f32>,
    /// Padding for alignment
    _pad0: f32,
    /// Light color/intensity
    light_color: vec3<f32>,
    /// Ambient light strength (0.0 - 1.0)
    ambient: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// ============================================================================
// Vertex Shader
// ============================================================================

/// Input vertex data from the vertex buffer.
struct VertexInput {
    /// World-space position (x, y, z)
    @location(0) position: vec3<f32>,
    /// RGB color (normalized 0-1)
    @location(1) color: vec3<f32>,
    /// Surface normal (normalized)
    @location(2) normal: vec3<f32>,
}

/// Output from vertex shader / input to fragment shader.
struct VertexOutput {
    /// Clip-space position (required builtin)
    @builtin(position) clip_position: vec4<f32>,
    /// Interpolated color passed to fragment shader
    @location(0) color: vec3<f32>,
    /// Interpolated normal for lighting calculation
    @location(1) normal: vec3<f32>,
}

/// Vertex shader entry point.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.normal = in.normal;
    return out;
}

// ============================================================================
// Fragment Shader
// ============================================================================

/// Fragment shader entry point with directional lighting.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the interpolated normal
    let normal = normalize(in.normal);

    // Calculate diffuse lighting (Lambert)
    let n_dot_l = max(dot(normal, uniforms.light_dir), 0.0);

    // Combine ambient and diffuse
    let diffuse = uniforms.light_color * n_dot_l;
    let lighting = uniforms.ambient + diffuse * (1.0 - uniforms.ambient);

    // Apply lighting to vertex color
    let lit_color = in.color * lighting;

    return vec4<f32>(lit_color, 1.0);
}
