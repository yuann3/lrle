// Terrain Wireframe Shader
//
// Simple vertex/fragment shader for rendering terrain wireframes.
// Receives position and color per vertex, applies view-projection transform,
// and outputs the color unchanged.

// ============================================================================
// Uniforms
// ============================================================================

/// Camera uniforms containing the combined view-projection matrix.
/// Updated each frame from the CPU.
struct Uniforms {
    /// Combined view * projection matrix for transforming world -> clip space
    view_proj: mat4x4<f32>,
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
}

/// Output from vertex shader / input to fragment shader.
struct VertexOutput {
    /// Clip-space position (required builtin)
    @builtin(position) clip_position: vec4<f32>,
    /// Interpolated color passed to fragment shader
    @location(0) color: vec3<f32>,
}

/// Vertex shader entry point.
///
/// Transforms vertex position from world space to clip space using the
/// view-projection matrix, and passes color through to the fragment shader.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

// ============================================================================
// Fragment Shader
// ============================================================================

/// Fragment shader entry point.
///
/// Simply outputs the interpolated vertex color with full opacity.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
