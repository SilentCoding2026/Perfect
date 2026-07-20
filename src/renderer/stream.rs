//! Streaming frame renderer — renders frames one at a time and passes them
//! to a callback, avoiding storing all frames in memory.

use std::collections::HashMap;

use crate::assets::AssetRegistry;
use crate::errors::AnimError;
use crate::scene::{EntityState, RenderConfig};
use crate::timeline::Timeline;

use super::{render_frame, Frame};

/// Render frames streaming to a callback.
///
/// Each frame is rendered and immediately passed to `frame_callback`,
/// which can write it to disk, encode it, or store it.
///
/// Returns the total number of frames rendered.
pub fn render_scene_stream<F>(
    config: &RenderConfig,
    timeline: &Timeline,
    initial_entities: &HashMap<String, EntityState>,
    set_name: Option<&str>,
    assets: &AssetRegistry,
    custom_poses: &HashMap<String, Vec<(String, f64)>>,
    mut frame_callback: F,
) -> Result<usize, AnimError>
where
    F: FnMut(&Frame) -> Result<(), AnimError>,
{
    use indicatif::{ProgressBar, ProgressStyle};

    let total_frames = (timeline.duration * config.fps as f64).ceil() as usize;

    log::info!(
        "Streaming {} frames ({}x{} @ {} fps, {:.1}s)",
        total_frames,
        config.width,
        config.height,
        config.fps,
        timeline.duration,
    );

    let pb = ProgressBar::new(total_frames as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} frames ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    for frame_idx in 0..total_frames {
        let t = frame_idx as f64 / config.fps as f64;
        let frame = render_frame(
            config,
            timeline,
            initial_entities,
            set_name,
            assets,
            custom_poses,
            t,
        )?;
        frame_callback(&frame)?;
        pb.inc(1);
    }

    pb.finish_with_message("Streaming complete");
    Ok(total_frames)
}

/// Render frames streaming to a file sink (MP4 via FFmpeg).
///
/// This is a convenience wrapper that spawns FFmpeg and pipes frames directly.
pub fn render_to_video<W: std::io::Write>(
    config: &RenderConfig,
    timeline: &Timeline,
    initial_entities: &HashMap<String, EntityState>,
    set_name: Option<&str>,
    assets: &AssetRegistry,
    custom_poses: &HashMap<String, Vec<(String, f64)>>,
    writer: &mut W,
) -> Result<usize, AnimError> {
    render_scene_stream(
        config,
        timeline,
        initial_entities,
        set_name,
        assets,
        custom_poses,
        |frame| {
            writer
                .write_all(&frame.data)
                .map_err(|e| AnimError::Video(format!("write error: {e}")))?;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::CameraTrack;

    #[test]
    fn test_streaming_rendering() {
        // Minimal test: ensure the streaming function doesn't panic with empty data.
        let config = RenderConfig::default();
        let timeline = Timeline {
            duration: 1.0,
            tracks: vec![],
            pose_events: vec![],
            camera_track: CameraTrack { keyframes: vec![] },
            transitions: vec![],
        };
        let entities = HashMap::new();
        let assets = AssetRegistry::new();
        let custom_poses = HashMap::new();

        let mut frames_rendered = 0;
        let result = render_scene_stream(
            &config,
            &timeline,
            &entities,
            None,
            &assets,
            &custom_poses,
            |_frame| {
                frames_rendered += 1;
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert_eq!(frames_rendered, 24); // 1 second at 24fps
    }
}
