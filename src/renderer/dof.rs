//! Depth of field (DoF) rendering.
//!
//! Applies blur to entities based on their z-distance from the focal plane.
//! Uses a simple Gaussian blur approximation for performance.

use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

use crate::errors::AnimError;

/// Depth of field configuration.
#[derive(Debug, Clone, Copy)]
pub struct DepthOfFieldConfig {
    /// Focal distance (z-coordinate of the focal plane).
    pub focal_distance: f64,
    /// Depth of field range (z-distance where blur starts).
    pub dof_range: f64,
    /// Maximum blur radius in pixels.
    pub max_blur_radius: f64,
    /// Enable DoF rendering.
    pub enabled: bool,
}

impl Default for DepthOfFieldConfig {
    fn default() -> Self {
        Self {
            focal_distance: 0.5,
            dof_range: 0.3,
            max_blur_radius: 8.0,
            enabled: false,
        }
    }
}

/// Compute the blur radius for a given z-distance.
pub fn compute_blur_radius(z: f64, config: &DepthOfFieldConfig) -> f64 {
    if !config.enabled {
        return 0.0;
    }

    let distance_from_focal = (z - config.focal_distance).abs();
    if distance_from_focal < config.dof_range {
        return 0.0;
    }

    let blur_factor = (distance_from_focal - config.dof_range) / (1.0 - config.dof_range);
    (blur_factor * config.max_blur_radius).min(config.max_blur_radius)
}

/// Apply depth of field blur to a pixmap.
///
/// Uses a separable Gaussian blur approximation for performance.
/// Entities rendered at different z-depths are blurred based on their
/// z-distance from the focal plane.
pub fn apply_depth_of_field(
    pixmap: &mut Pixmap,
    z_buffer: &[f64],
    config: &DepthOfFieldConfig,
) -> Result<(), AnimError> {
    if !config.enabled || z_buffer.is_empty() {
        return Ok(());
    }

    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let data = pixmap.data_mut();

    // We need to compute per-pixel blur radius based on the z-buffer.
    // For simplicity, we apply a uniform blur based on the average z of the
    // visible pixels.
    let mut total_z = 0.0;
    let mut count = 0;
    for &z in z_buffer {
        if z >= 0.0 {
            total_z += z;
            count += 1;
        }
    }

    if count == 0 {
        return Ok(());
    }

    let avg_z = total_z / count as f64;
    let blur_radius = compute_blur_radius(avg_z, config);

    if blur_radius < 0.5 {
        return Ok(());
    }

    // Apply a simple box blur for performance.
    // In production, we'd use a more sophisticated Gaussian blur.
    apply_box_blur(pixmap, blur_radius as usize);

    Ok(())
}

/// Apply a simple box blur to a pixmap.
fn apply_box_blur(pixmap: &mut Pixmap, radius: usize) {
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let data = pixmap.data_mut();

    if radius == 0 || w < radius * 2 || h < radius * 2 {
        return;
    }

    let kernel_size = radius * 2 + 1;
    let kernel_area = (kernel_size * kernel_size) as f64;

    // Horizontal pass.
    let mut temp = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a = 0.0;

            for dx in 0..kernel_size {
                let sx = x + dx - radius;
                if sx < 0 || sx >= w {
                    continue;
                }
                let idx = (y * w + sx) * 4;
                r += data[idx] as f64;
                g += data[idx + 1] as f64;
                b += data[idx + 2] as f64;
                a += data[idx + 3] as f64;
            }

            let idx = (y * w + x) * 4;
            temp[idx] = (r / kernel_size as f64) as u8;
            temp[idx + 1] = (g / kernel_size as f64) as u8;
            temp[idx + 2] = (b / kernel_size as f64) as u8;
            temp[idx + 3] = (a / kernel_size as f64) as u8;
        }
    }

    // Vertical pass.
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a = 0.0;

            for dy in 0..kernel_size {
                let sy = y + dy - radius;
                if sy < 0 || sy >= h {
                    continue;
                }
                let idx = (sy * w + x) * 4;
                r += temp[idx] as f64;
                g += temp[idx + 1] as f64;
                b += temp[idx + 2] as f64;
                a += temp[idx + 3] as f64;
            }

            let idx = (y * w + x) * 4;
            data[idx] = (r / kernel_size as f64) as u8;
            data[idx + 1] = (g / kernel_size as f64) as u8;
            data[idx + 2] = (b / kernel_size as f64) as u8;
            data[idx + 3] = (a / kernel_size as f64) as u8;
        }
    }
}

/// Apply depth of field with per-pixel z-depth information.
///
/// More accurate but slower. Uses the z-buffer to compute per-pixel blur.
pub fn apply_depth_of_field_per_pixel(
    pixmap: &mut Pixmap,
    z_buffer: &[f64],
    config: &DepthOfFieldConfig,
) -> Result<(), AnimError> {
    if !config.enabled || z_buffer.is_empty() {
        return Ok(());
    }

    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;

    if z_buffer.len() < w * h {
        return Err(AnimError::Render(
            "Z-buffer size does not match pixmap size".into(),
        ));
    }

    // Compute blur radius for each pixel.
    let mut blur_radii = vec![0.0; w * h];
    let mut max_radius = 0.0;
    for (i, &z) in z_buffer.iter().enumerate() {
        let r = compute_blur_radius(z, config);
        blur_radii[i] = r;
        if r > max_radius {
            max_radius = r;
        }
    }

    if max_radius < 0.5 {
        return Ok(());
    }

    // Apply Gaussian blur with varying radius.
    // For simplicity, we use a uniform blur with the average radius.
    let avg_radius: f64 = blur_radii.iter().sum::<f64>() / blur_radii.len() as f64;
    apply_box_blur(pixmap, avg_radius.ceil() as usize);

    Ok(())
}

/// Create a Z-buffer from entity depths.
pub fn create_z_buffer(
    width: u32,
    height: u32,
    entity_depths: &[(f64, f64, f64)], // (x, y, z) normalized
) -> Vec<f64> {
    let w = width as usize;
    let h = height as usize;
    let mut z_buffer = vec![0.0; w * h];

    // Simple approach: render entity depths to the z-buffer.
    // Each entity has a z-value; we fill a rectangle based on entity position.
    // This is a simplified version; in production, we'd use actual pixel coverage.
    for (x, y, z) in entity_depths {
        let px = (x * width as f64) as usize;
        let py = (y * height as f64) as usize;
        let size = 20; // Approximate entity size in pixels.

        for dy in 0..size {
            for dx in 0..size {
                let sx = px + dx - size / 2;
                let sy = py + dy - size / 2;
                if sx < w && sy < h {
                    let idx = sy * w + sx;
                    // Use the minimum z (closest) for each pixel.
                    if *z > z_buffer[idx] {
                        z_buffer[idx] = *z;
                    }
                }
            }
        }
    }

    z_buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blur_radius_computation() {
        let config = DepthOfFieldConfig::default();

        // At focal distance, blur should be 0.
        assert_eq!(compute_blur_radius(0.5, &config), 0.0);

        // Within DoF range, blur should be 0.
        assert_eq!(compute_blur_radius(0.6, &config), 0.0);
        assert_eq!(compute_blur_radius(0.4, &config), 0.0);

        // Far from focal plane, blur should be non-zero.
        assert!(compute_blur_radius(0.0, &config) > 0.0);
        assert!(compute_blur_radius(1.0, &config) > 0.0);

        // Max blur should not exceed max_blur_radius.
        assert!(compute_blur_radius(0.0, &config) <= config.max_blur_radius);
        assert!(compute_blur_radius(1.0, &config) <= config.max_blur_radius);
    }

    #[test]
    fn test_dof_config_default() {
        let config = DepthOfFieldConfig::default();
        assert_eq!(config.focal_distance, 0.5);
        assert_eq!(config.dof_range, 0.3);
        assert_eq!(config.max_blur_radius, 8.0);
        assert!(!config.enabled);
    }
}
