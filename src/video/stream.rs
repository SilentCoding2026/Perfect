//! Streaming video encoder — encodes frames as they are produced,
//! avoiding storing all frames in memory.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::AnimError;
use crate::renderer::Frame;

/// Streaming video encoder that writes frames to FFmpeg as they arrive.
pub struct VideoStreamEncoder {
    child: std::process::Child,
    width: u32,
    height: u32,
    _fps: u32,
    frame_count: u64,
    output_path: std::path::PathBuf,
}

impl VideoStreamEncoder {
    /// Create a new streaming encoder.
    ///
    /// Spawns an FFmpeg process and prepares to receive frames.
    pub fn new(output: &Path, width: u32, height: u32, fps: u32) -> Result<Self, AnimError> {
        // Create parent directories if they don't exist.
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
            "Starting streaming encoder: {}x{} @ {} fps -> {}",
            width,
            height,
            fps,
            output.display()
        );

        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                &format!("{width}x{height}"),
                "-r",
                &fps.to_string(),
                "-i",
                "-",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-preset",
                "medium",
                "-crf",
                "23",
                "-movflags",
                "+faststart",
            ])
            .arg(output.as_os_str())
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
            _fps: fps,
            frame_count: 0,
            output_path: output.to_path_buf(),
        })
    }

    /// Write a single frame to the encoder.
    ///
    /// Returns the frame index after writing.
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

    /// Flush and close the encoder, waiting for FFmpeg to finish.
    pub fn finish(mut self) -> Result<u64, AnimError> {
        // Close stdin to signal end of input.
        drop(self.child.stdin.take());

        log::info!("Waiting for FFmpeg to finish encoding...");

        let status = self
            .child
            .wait()
            .map_err(|e| AnimError::Video(format!("ffmpeg process error: {e}")))?;

        if !status.success() {
            return Err(AnimError::Video(format!(
                "ffmpeg exited with status: {status}"
            )));
        }

        log::info!(
            "Video encoded successfully: {} ({} frames)",
            self.output_path.display(),
            self.frame_count
        );

        Ok(self.frame_count)
    }

    /// Get the number of frames written so far.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get the output path.
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

impl Drop for VideoStreamEncoder {
    fn drop(&mut self) {
        // Try to close stdin and wait for the child if it's still running.
        // We can't do much if this fails; it's a best-effort cleanup.
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.flush();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Encode frames as they're produced using the streaming encoder.
///
/// This is a convenience wrapper that creates an encoder and passes it to
/// a frame-producing callback. The callback receives a mutable reference
/// to the encoder and writes frames as they're generated.
pub fn encode_streaming<F>(
    output: &Path,
    width: u32,
    height: u32,
    fps: u32,
    mut frame_producer: F,
) -> Result<u64, AnimError>
where
    F: FnMut(&mut VideoStreamEncoder) -> Result<(), AnimError>,
{
    let mut encoder = VideoStreamEncoder::new(output, width, height, fps)?;
    frame_producer(&mut encoder)?;
    encoder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_lifecycle() {
        // This test creates and drops an encoder without writing any frames.
        let temp_dir = std::env::temp_dir();
        let output = temp_dir.join("test_encoder.mp4");

        let result = VideoStreamEncoder::new(&output, 640, 480, 24);
        // If ffmpeg is not installed, this will fail. We just check it doesn't panic.
        match result {
            Ok(encoder) => {
                let _ = encoder.finish();
            }
            Err(e) => {
                // FFmpeg not available — skip test.
                eprintln!("Skipping encoder test: {e}");
            }
        }

        // Clean up.
        let _ = std::fs::remove_file(&output);
    }
}
