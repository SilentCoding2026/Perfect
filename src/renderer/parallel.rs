//! Parallel frame rendering — renders frames across multiple CPU cores.
//!
//! Uses rayon for work-stealing parallelism. Each frame is rendered
//! independently, then collected and sorted by frame index.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rayon::prelude::*;

use crate::assets::AssetRegistry;
use crate::errors::AnimError;
use crate::scene::{EntityState, RenderConfig};
use crate::timeline::Timeline;

use super::{render_frame, Frame};

/// Render frames in parallel using rayon.
///
/// Each frame is rendered independently on a thread pool.
/// Results are collected and sorted by frame index.
pub fn render_scene_parallel(
    config: &RenderConfig,
    timeline: &Timeline,
    initial_entities: &HashMap<String, EntityState>,
    set_name: Option<&str>,
    assets: &AssetRegistry,
    custom_poses: &HashMap<String, Vec<(String, f64)>>,
) -> Result<Vec<Frame>, AnimError> {
    use indicatif::{ProgressBar, ProgressStyle};

    let total_frames = (timeline.duration * config.fps as f64).ceil() as usize;

    log::info!(
        "Parallel rendering {} frames ({}x{} @ {} fps, {:.1}s) using {} cores",
        total_frames,
        config.width,
        config.height,
        config.fps,
        timeline.duration,
        rayon::current_num_threads(),
    );

    // Wrap assets in Arc for sharing across threads.
    // We need to clone the asset data for each thread since we can't share
    // references across threads safely without Arc.
    let assets_arc = Arc::new(assets);
    let initial_entities_arc = Arc::new(initial_entities.clone());
    let custom_poses_arc = Arc::new(custom_poses.clone());

    let pb = ProgressBar::new(total_frames as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} frames ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let pb_arc = Arc::new(Mutex::new(pb));

    // Render frames in parallel, preserving frame indices for correct ordering.
    let frames_with_index: Vec<(usize, Result<Frame, AnimError>)> = (0..total_frames)
        .into_par_iter()
        .map(|frame_idx| {
            let t = frame_idx as f64 / config.fps as f64;

            // Clone data for this thread.
            let assets = assets_arc.as_ref();
            let initial_entities = initial_entities_arc.as_ref().clone();
            let custom_poses = custom_poses_arc.as_ref().clone();

            let result = render_frame(
                config,
                timeline,
                &initial_entities,
                set_name,
                assets,
                &custom_poses,
                t,
            );

            // Update progress bar.
            if let Ok(pb) = pb_arc.lock() {
                pb.inc(1);
            }

            (frame_idx, result)
        })
        .collect();

    // Check for failures and collect successes.
    let mut successes = Vec::with_capacity(total_frames);
    for (idx, result) in frames_with_index {
        match result {
            Ok(frame) => successes.push((idx, frame)),
            Err(e) => {
                log::error!("Frame {} failed: {}", idx, e);
                return Err(AnimError::Render(format!("Frame {} failed: {}", idx, e)));
            }
        }
    }

    // Sort by index (rayon may reorder).
    successes.sort_by_key(|(idx, _)| *idx);

    // Extract frames.
    let result: Vec<Frame> = successes.into_iter().map(|(_, frame)| frame).collect();

    if let Ok(pb) = pb_arc.lock() {
        pb.finish_with_message("Parallel render complete");
    }

    // Log cache stats.
    if let Ok(cache) = super::POSE_CACHE.lock() {
        let (hits, misses) = cache.stats();
        if hits + misses > 0 {
            log::info!(
                "Pose cache: {} hits, {} misses ({:.1}% hit rate)",
                hits,
                misses,
                (hits as f64 / (hits + misses) as f64) * 100.0
            );
        }
    }

    Ok(result)
}

/// Get the number of available CPU cores for parallel rendering.
pub fn num_cores() -> usize {
    rayon::current_num_threads()
}

/// Render frames with adaptive parallelism based on frame complexity.
///
/// For short animations (< 100 frames), use sequential rendering to avoid
/// overhead. For longer animations, use parallel rendering.
pub fn render_scene_adaptive(
    config: &RenderConfig,
    timeline: &Timeline,
    initial_entities: &HashMap<String, EntityState>,
    set_name: Option<&str>,
    assets: &AssetRegistry,
    custom_poses: &HashMap<String, Vec<(String, f64)>>,
    min_frames_for_parallel: usize,
) -> Result<Vec<Frame>, AnimError> {
    let total_frames = (timeline.duration * config.fps as f64).ceil() as usize;

    if total_frames < min_frames_for_parallel {
        log::info!(
            "Using sequential render ({} frames < threshold)",
            total_frames
        );
        super::render_scene(
            config,
            timeline,
            initial_entities,
            set_name,
            assets,
            custom_poses,
        )
    } else {
        render_scene_parallel(
            config,
            timeline,
            initial_entities,
            set_name,
            assets,
            custom_poses,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::EntityKind;
    use crate::timeline::CameraTrack;

    #[test]
    fn test_parallel_rendering_basic() {
        let config = RenderConfig::default();
        let timeline = Timeline {
            duration: 0.5,
            tracks: vec![],
            pose_events: vec![],
            camera_track: CameraTrack { keyframes: vec![] },
            transitions: vec![],
        };
        let entities = HashMap::new();
        let assets = AssetRegistry::new();
        let custom_poses = HashMap::new();

        let result =
            render_scene_parallel(&config, &timeline, &entities, None, &assets, &custom_poses);

        // Should render 12 frames (0.5s at 24fps).
        assert!(result.is_ok());
        if let Ok(frames) = result {
            assert_eq!(frames.len(), 12);
        }
    }

    #[test]
    fn test_adaptive_rendering() {
        let config = RenderConfig::default();
        let timeline = Timeline {
            duration: 0.5,
            tracks: vec![],
            pose_events: vec![],
            camera_track: CameraTrack { keyframes: vec![] },
            transitions: vec![],
        };
        let entities = HashMap::new();
        let assets = AssetRegistry::new();
        let custom_poses = HashMap::new();

        // With threshold 100, should use sequential (12 < 100).
        let result = render_scene_adaptive(
            &config,
            &timeline,
            &entities,
            None,
            &assets,
            &custom_poses,
            100,
        );
        assert!(result.is_ok());

        // With threshold 5, should use parallel (12 >= 5).
        let result = render_scene_adaptive(
            &config,
            &timeline,
            &entities,
            None,
            &assets,
            &custom_poses,
            5,
        );
        assert!(result.is_ok());
    }
}
