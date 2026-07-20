//! Video pipeline — encodes rendered frames into video files via FFmpeg.

pub mod formats;
pub mod stream;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::AnimError;
use crate::renderer::Frame;

// Re-export streaming and format types.
pub use formats::{VideoFormat, StreamingFormatEncoder};
pub use stream::VideoStreamEncoder;

/// Encode a sequence of frames into an MP4 video file.
pub fn encode_video(frames: &[Frame], output: &Path, fps: u32) -> Result<(), AnimError> {
    formats::encode_video_with_format(frames, output, fps, VideoFormat::Mp4)
}

/// Encode frames as individual PNG files (useful for debugging).
pub fn encode_png_sequence(frames: &[Frame], output_dir: &Path) -> Result<(), AnimError> {
    std::fs::create_dir_all(output_dir)?;

    for (i, frame) in frames.iter().enumerate() {
        let path = output_dir.join(format!("frame_{:06}.png", i));
        write_png(&path, &frame.data, frame.width, frame.height)?;
    }

    log::info!(
        "Wrote {} PNG frames to {}",
        frames.len(),
        output_dir.display()
    );
    Ok(())
}

/// Encode frames streaming to video without storing all frames in memory.
pub fn encode_video_streaming<F>(
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

/// Encode frames streaming with a specific video format.
pub fn encode_video_streaming_with_format<F>(
    output: &Path,
    width: u32,
    height: u32,
    fps: u32,
    format: VideoFormat,
    mut frame_producer: F,
) -> Result<u64, AnimError>
where
    F: FnMut(&mut StreamingFormatEncoder) -> Result<(), AnimError>,
{
    let mut encoder = StreamingFormatEncoder::new(output, width, height, fps, format)?;
    frame_producer(&mut encoder)?;
    encoder.finish()
}

fn write_png(path: &Path, data: &[u8], width: u32, height: u32) -> Result<(), AnimError> {
    let file = std::fs::File::create(path)?;
    let w = std::io::BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|e| AnimError::Render(format!("PNG header error: {e}")))?;

    writer
        .write_image_data(data)
        .map_err(|e| AnimError::Render(format!("PNG write error: {e}")))?;

    Ok(())
}