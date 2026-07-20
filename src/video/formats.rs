//! Additional video export formats.
//!
//! Supports: GIF, WebM, MOV (QuickTime), and image sequences.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::AnimError;
use crate::renderer::Frame;

/// Video format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    /// H.264 MP4 (default)
    Mp4,
    /// WebM (VP9)
    WebM,
    /// QuickTime MOV (ProRes)
    Mov,
    /// Animated GIF
    Gif,
}

impl VideoFormat {
    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::WebM => "webm",
            VideoFormat::Mov => "mov",
            VideoFormat::Gif => "gif",
        }
    }

    /// Get the FFmpeg codec for this format.
    pub fn codec(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "libx264",
            VideoFormat::WebM => "libvpx-vp9",
            VideoFormat::Mov => "prores_ks",
            VideoFormat::Gif => "gif",
        }
    }

    /// Get the FFmpeg pixel format for this format.
    pub fn pixel_format(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "yuv420p",
            VideoFormat::WebM => "yuv420p",
            VideoFormat::Mov => "yuv422p10le",
            VideoFormat::Gif => "rgb8",
        }
    }

    /// Get additional FFmpeg arguments for this format.
    pub fn extra_args(&self) -> Vec<String> {
        match self {
            VideoFormat::Mp4 => vec![
                "-movflags".to_string(),
                "+faststart".to_string(),
                "-crf".to_string(),
                "23".to_string(),
            ],
            VideoFormat::WebM => vec![
                "-crf".to_string(),
                "30".to_string(),
                "-b:v".to_string(),
                "0".to_string(),
            ],
            VideoFormat::Mov => vec![
                "-profile:v".to_string(),
                "4444".to_string(),
                "-vendor".to_string(),
                "ap10".to_string(),
            ],
            VideoFormat::Gif => vec![
                "-filter_complex".to_string(),
                "[0:v]split[a][b];[a]palettegen[p];[b][p]paletteuse".to_string(),
            ],
        }
    }

    /// Check if the format requires palette generation (GIF).
    pub fn requires_palette(&self) -> bool {
        matches!(self, VideoFormat::Gif)
    }

    /// Check if the format supports alpha channel.
    pub fn supports_alpha(&self) -> bool {
        matches!(self, VideoFormat::Mov)
    }
}

impl From<&str> for VideoFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mp4" | "h264" => VideoFormat::Mp4,
            "webm" | "vp9" => VideoFormat::WebM,
            "mov" | "quicktime" | "prores" => VideoFormat::Mov,
            "gif" => VideoFormat::Gif,
            _ => VideoFormat::Mp4,
        }
    }
}

/// Encode frames to a video file with the specified format.
pub fn encode_video_with_format(
    frames: &[Frame],
    output: &Path,
    fps: u32,
    format: VideoFormat,
) -> Result<(), AnimError> {
    if frames.is_empty() {
        return Err(AnimError::Video("no frames to encode".into()));
    }

    let width = frames[0].width;
    let height = frames[0].height;

    log::info!(
        "Encoding {} frames to {} ({}x{} @ {} fps, format: {:?})",
        frames.len(),
        output.display(),
        width,
        height,
        fps,
        format,
    );

    // Create parent directories.
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AnimError::Video(format!(
                    "failed to create output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }
    }

    // Build FFmpeg command.
    let mut args = vec![
        "-y".to_string(), // overwrite
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-s".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "-".to_string(),
        "-c:v".to_string(),
        format.codec().to_string(),
        "-pix_fmt".to_string(),
        format.pixel_format().to_string(),
    ];

    // Add format-specific arguments.
    args.extend(format.extra_args());

    // Add output path.
    args.push(output.as_os_str().to_string_lossy().to_string());

    log::debug!("FFmpeg command: ffmpeg {}", args.join(" "));

    // Spawn FFmpeg process.
    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AnimError::Video(format!("failed to start ffmpeg: {e}. Is ffmpeg installed?"))
        })?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| AnimError::Video("failed to open ffmpeg stdin".into()))?;

    // Write frames.
    for (i, frame) in frames.iter().enumerate() {
        stdin
            .write_all(&frame.data)
            .map_err(|e| AnimError::Video(format!("failed to write frame {i} to ffmpeg: {e}")))?;
    }

    // Close stdin.
    drop(child.stdin.take());

    // Wait for completion.
    let output_result = child
        .wait_with_output()
        .map_err(|e| AnimError::Video(format!("ffmpeg process error: {e}")))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(AnimError::Video(format!("ffmpeg failed: {stderr}")));
    }

    log::info!("Video encoded successfully: {}", output.display());
    Ok(())
}

/// Encode frames to a GIF with custom settings.
pub fn encode_gif(
    frames: &[Frame],
    output: &Path,
    fps: u32,
    dither: bool,
) -> Result<(), AnimError> {
    if frames.is_empty() {
        return Err(AnimError::Video("no frames to encode".into()));
    }

    let width = frames[0].width;
    let height = frames[0].height;

    log::info!(
        "Encoding {} frames to GIF {} ({}x{} @ {} fps)",
        frames.len(),
        output.display(),
        width,
        height,
        fps,
    );

    // Create parent directories.
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AnimError::Video(format!(
                    "failed to create output directory '{}': {e}",
                    parent.display()
                ))
            })?;
        }
    }

    // Build the palette generation command for better quality.
    let palette_filter = if dither {
        "paletteuse=dither=sierra2_4a"
    } else {
        "paletteuse"
    };

    let mut args = vec![
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-s".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "-".to_string(),
        "-filter_complex".to_string(),
        format!(
            "[0:v]split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]{}",
            palette_filter
        ),
        "-c:v".to_string(),
        "gif".to_string(),
    ];

    // Add output path.
    args.push(output.as_os_str().to_string_lossy().to_string());

    // Spawn FFmpeg.
    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AnimError::Video(format!(
                "failed to start ffmpeg for GIF: {e}. Is ffmpeg installed?"
            ))
        })?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| AnimError::Video("failed to open ffmpeg stdin".into()))?;

    // Write frames.
    for (i, frame) in frames.iter().enumerate() {
        stdin
            .write_all(&frame.data)
            .map_err(|e| AnimError::Video(format!("failed to write frame {i} to ffmpeg: {e}")))?;
    }

    drop(child.stdin.take());

    let output_result = child
        .wait_with_output()
        .map_err(|e| AnimError::Video(format!("ffmpeg process error: {e}")))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(AnimError::Video(format!(
            "ffmpeg GIF encoding failed: {stderr}"
        )));
    }

    log::info!("GIF encoded successfully: {}", output.display());
    Ok(())
}

/// Streaming encoder with format support.
pub struct StreamingFormatEncoder {
    child: std::process::Child,
    width: u32,
    height: u32,
    fps: u32,
    frame_count: u64,
    format: VideoFormat,
    output_path: std::path::PathBuf,
}

impl StreamingFormatEncoder {
    /// Create a new streaming encoder for the specified format.
    pub fn new(
        output: &Path,
        width: u32,
        height: u32,
        fps: u32,
        format: VideoFormat,
    ) -> Result<Self, AnimError> {
        // Create parent directories.
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AnimError::Video(format!(
                        "failed to create output directory '{}': {e}",
                        parent.display()
                    ))
                })?;
            }
        }

        log::info!(
            "Starting streaming encoder: {}x{} @ {} fps, format: {:?} -> {}",
            width,
            height,
            fps,
            format,
            output.display()
        );

        let mut args = vec![
            "-y".to_string(),
            "-f".to_string(),
            "rawvideo".to_string(),
            "-pix_fmt".to_string(),
            "rgba".to_string(),
            "-s".to_string(),
            format!("{width}x{height}"),
            "-r".to_string(),
            fps.to_string(),
            "-i".to_string(),
            "-".to_string(),
            "-c:v".to_string(),
            format.codec().to_string(),
            "-pix_fmt".to_string(),
            format.pixel_format().to_string(),
        ];

        args.extend(format.extra_args());
        args.push(output.as_os_str().to_string_lossy().to_string());

        let child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                AnimError::Video(format!("failed to start ffmpeg: {e}. Is ffmpeg installed?"))
            })?;

        Ok(Self {
            child,
            width,
            height,
            fps,
            frame_count: 0,
            format,
            output_path: output.to_path_buf(),
        })
    }

    /// Write a single frame.
    pub fn write_frame(&mut self, frame: &Frame) -> Result<u64, AnimError> {
        if frame.width != self.width || frame.height != self.height {
            return Err(AnimError::Video(format!(
                "Frame size mismatch: expected {}x{}, got {}x{}",
                self.width, self.height, frame.width, frame.height
            )));
        }

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| AnimError::Video("ffmpeg stdin not available".into()))?;

        stdin.write_all(&frame.data).map_err(|e| {
            AnimError::Video(format!("failed to write frame {}: {e}", self.frame_count))
        })?;

        self.frame_count += 1;
        Ok(self.frame_count - 1)
    }

    /// Finish encoding.
    pub fn finish(mut self) -> Result<u64, AnimError> {
        drop(self.child.stdin.take());

        log::info!("Waiting for FFmpeg to finish encoding...");

        let output_result = self
            .child
            .wait_with_output()
            .map_err(|e| AnimError::Video(format!("ffmpeg process error: {e}")))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(AnimError::Video(format!("ffmpeg failed: {stderr}")));
        }

        log::info!(
            "Video encoded successfully: {} ({} frames, format: {:?})",
            self.output_path.display(),
            self.frame_count,
            self.format
        );

        Ok(self.frame_count)
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn format(&self) -> VideoFormat {
        self.format
    }
}

impl Drop for StreamingFormatEncoder {
    fn drop(&mut self) {
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.flush();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_format_from_str() {
        assert_eq!(VideoFormat::from("mp4"), VideoFormat::Mp4);
        assert_eq!(VideoFormat::from("h264"), VideoFormat::Mp4);
        assert_eq!(VideoFormat::from("webm"), VideoFormat::WebM);
        assert_eq!(VideoFormat::from("mov"), VideoFormat::Mov);
        assert_eq!(VideoFormat::from("gif"), VideoFormat::Gif);
        assert_eq!(VideoFormat::from("unknown"), VideoFormat::Mp4);
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(VideoFormat::Mp4.extension(), "mp4");
        assert_eq!(VideoFormat::WebM.extension(), "webm");
        assert_eq!(VideoFormat::Mov.extension(), "mov");
        assert_eq!(VideoFormat::Gif.extension(), "gif");
    }
}
