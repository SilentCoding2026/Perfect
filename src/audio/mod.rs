//! Audio support for AnimDSL.
//!
//! Provides audio track loading, timeline synchronization, and
//! audio-video muxing via FFmpeg.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::AnimError;

/// Audio track loaded from a file.
#[derive(Debug, Clone)]
pub struct AudioTrack {
    /// Path to the audio file.
    pub path: PathBuf,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
    /// Duration in seconds.
    pub duration: f64,
    /// Format (mp3, wav, aac, etc.)
    pub format: String,
}

/// Audio timeline — maps audio segments to timeline positions.
#[derive(Debug, Clone)]
pub struct AudioTimeline {
    /// Main audio track (background music).
    pub main_track: Option<AudioTrack>,
    /// Audio cues at specific times.
    pub cues: Vec<AudioCue>,
    /// Audio volume (0.0 to 1.0).
    pub volume: f64,
}

/// An audio cue at a specific time.
#[derive(Debug, Clone)]
pub struct AudioCue {
    /// Time in seconds when the cue starts.
    pub time: f64,
    /// Audio track for this cue.
    pub track: AudioTrack,
    /// Duration of the cue (None = plays until end).
    pub duration: Option<f64>,
    /// Volume for this cue (0.0 to 1.0).
    pub volume: f64,
}

impl AudioTrack {
    /// Load an audio file and read its metadata.
    pub fn load(path: &Path) -> Result<Self, AnimError> {
        if !path.exists() {
            return Err(AnimError::Audio(format!(
                "Audio file not found: {}",
                path.display()
            )));
        }

        // Use ffprobe to get audio metadata.
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                path.as_os_str().to_str().unwrap(),
            ])
            .output()
            .map_err(|e| AnimError::Audio(format!("Failed to probe audio file: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AnimError::Audio(format!(
                "ffprobe failed for {}: {}",
                path.display(),
                stderr
            )));
        }

        // Parse ffprobe JSON output.
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| AnimError::Audio(format!("Failed to parse ffprobe output: {}", e)))?;

        let format = json["format"]["format_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let duration = json["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        // Get sample rate and channels from the first audio stream.
        let empty = vec![];
        let streams = json["streams"].as_array().unwrap_or(&empty);
        let audio_stream = streams
            .iter()
            .find(|s| s["codec_type"].as_str() == Some("audio"));

        let sample_rate = audio_stream
            .and_then(|s| s["sample_rate"].as_str())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(44100);

        let channels = audio_stream
            .and_then(|s| s["channels"].as_u64())
            .unwrap_or(2) as u16;

        Ok(AudioTrack {
            path: path.to_path_buf(),
            sample_rate,
            channels,
            duration,
            format,
        })
    }

    /// Get the file extension.
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Check if the audio format is supported by FFmpeg.
    pub fn is_supported(&self) -> bool {
        matches!(
            self.extension(),
            Some("mp3") | Some("wav") | Some("aac") | Some("flac") | Some("ogg") | Some("m4a")
        )
    }
}

impl AudioTimeline {
    /// Create a new audio timeline.
    pub fn new() -> Self {
        Self {
            main_track: None,
            cues: Vec::new(),
            volume: 1.0,
        }
    }

    /// Set the main audio track.
    pub fn set_main_track(&mut self, path: &Path) -> Result<(), AnimError> {
        let track = AudioTrack::load(path)?;
        self.main_track = Some(track);
        Ok(())
    }

    /// Add an audio cue at a specific time.
    pub fn add_cue(
        &mut self,
        time: f64,
        path: &Path,
        duration: Option<f64>,
        volume: f64,
    ) -> Result<(), AnimError> {
        let track = AudioTrack::load(path)?;
        self.cues.push(AudioCue {
            time,
            track,
            duration,
            volume: volume.clamp(0.0, 1.0),
        });
        // Sort cues by time.
        self.cues
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        Ok(())
    }

    /// Get the audio track at a specific time.
    pub fn get_track_at_time(&self, t: f64) -> Option<&AudioTrack> {
        // Check cues first.
        for cue in &self.cues {
            if t >= cue.time {
                if let Some(dur) = cue.duration {
                    if t < cue.time + dur {
                        return Some(&cue.track);
                    }
                } else {
                    return Some(&cue.track);
                }
            }
        }
        // Fall back to main track.
        self.main_track.as_ref()
    }

    /// Get the total duration of the audio timeline.
    pub fn duration(&self) -> f64 {
        let mut max_dur = self.main_track.as_ref().map(|t| t.duration).unwrap_or(0.0);
        for cue in &self.cues {
            let end = cue.time + cue.duration.unwrap_or(cue.track.duration);
            if end > max_dur {
                max_dur = end;
            }
        }
        max_dur
    }
}

impl Default for AudioTimeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Mux audio and video streams into a single file.
pub fn mux_audio_video(
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> Result<(), AnimError> {
    log::info!(
        "Muxing audio {} with video {} -> {}",
        audio_path.display(),
        video_path.display(),
        output_path.display()
    );

    // Create parent directories.
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AnimError::Audio(format!(
                    "failed to create output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }
    }

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            video_path.as_os_str().to_str().unwrap(),
            "-i",
            audio_path.as_os_str().to_str().unwrap(),
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-shortest",
            output_path.as_os_str().to_str().unwrap(),
        ])
        .status()
        .map_err(|e| AnimError::Audio(format!("Failed to run ffmpeg: {}", e)))?;

    if !status.success() {
        return Err(AnimError::Audio(format!(
            "FFmpeg muxing failed for {}",
            output_path.display()
        )));
    }

    log::info!("Muxing complete: {}", output_path.display());
    Ok(())
}

/// Extract audio from a video file.
pub fn extract_audio(input_path: &Path, output_path: &Path) -> Result<(), AnimError> {
    log::info!(
        "Extracting audio from {} -> {}",
        input_path.display(),
        output_path.display()
    );

    // Create parent directories.
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AnimError::Audio(format!(
                    "failed to create output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }
    }

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input_path.as_os_str().to_str().unwrap(),
            "-vn",
            "-acodec",
            "copy",
            output_path.as_os_str().to_str().unwrap(),
        ])
        .status()
        .map_err(|e| AnimError::Audio(format!("Failed to run ffmpeg: {}", e)))?;

    if !status.success() {
        return Err(AnimError::Audio(format!(
            "FFmpeg audio extraction failed for {}",
            input_path.display()
        )));
    }

    log::info!("Audio extraction complete: {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_timeline_creation() {
        let mut timeline = AudioTimeline::new();
        assert!(timeline.main_track.is_none());
        assert_eq!(timeline.cues.len(), 0);
        assert_eq!(timeline.volume, 1.0);
    }

    #[test]
    fn test_audio_timeline_duration() {
        let mut timeline = AudioTimeline::new();
        // Add a cue with a duration.
        // Since we can't load real audio files in tests, we'll just check that
        // the duration is computed correctly from the stored values.
        // This test is more of a placeholder.
        assert_eq!(timeline.duration(), 0.0);
    }
}
