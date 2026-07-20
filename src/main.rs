//! AnimDSL — CLI entry point.
//!
//! Usage:
//!   animdsl render scene.anim -o output.mp4
//!   animdsl render scene.anim -o output.gif --format gif
//!   animdsl render scene.anim --format webm -o output.webm
//!   animdsl render scene.anim --png-dir ./frames
//!   animdsl check scene.anim

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};

use animdsl::assets::AssetRegistry;
use animdsl::ast::TopLevelItem;
use animdsl::errors::AnimError;
use animdsl::renderer;
use animdsl::scene::{resolve_scene, EntityKind, RenderConfig};
use animdsl::timeline;
use animdsl::video;
use animdsl::video::VideoFormat;

#[derive(Parser)]
#[command(
    name = "animdsl",
    version,
    about = "A DSL for generating 2D animated movie scenes"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a .anim file to video or image sequence.
    Render {
        /// Path to the .anim source file.
        input: PathBuf,

        /// Output video file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output directory for PNG frame sequence (alternative to video).
        #[arg(long)]
        png_dir: Option<PathBuf>,

        /// Video format: mp4, webm, mov, gif.
        #[arg(long, default_value = "mp4")]
        format: String,

        /// Override FPS.
        #[arg(long)]
        fps: Option<u32>,

        /// Override width.
        #[arg(long)]
        width: Option<u32>,

        /// Override height.
        #[arg(long)]
        height: Option<u32>,

        /// Disable parallel rendering (use single-threaded).
        #[arg(long)]
        sequential: bool,
    },

    /// Parse and validate a .anim file without rendering.
    Check {
        /// Path to the .anim source file.
        input: PathBuf,
    },

    /// Parse a .anim file and dump the AST as JSON.
    Dump {
        /// Path to the .anim source file.
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Render {
            input,
            output,
            png_dir,
            format,
            fps,
            width,
            height,
            sequential,
        } => {
            let video_format = VideoFormat::from(format.as_str());
            let output = output.unwrap_or_else(|| {
                let ext = video_format.extension();
                input.with_extension(ext)
            });
            cmd_render(
                &input,
                &output,
                png_dir.as_deref(),
                video_format,
                fps,
                width,
                height,
                sequential,
            )?;
        }
        Commands::Check { input } => {
            cmd_check(&input)?;
        }
        Commands::Dump { input } => {
            cmd_dump(&input)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_render(
    input: &Path,
    output: &Path,
    png_dir: Option<&Path>,
    format: VideoFormat,
    fps_override: Option<u32>,
    width_override: Option<u32>,
    height_override: Option<u32>,
    sequential: bool,
) -> Result<()> {
    let source = std::fs::read_to_string(input)?;
    let base_dir = input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Parse.
    let program = animdsl::parser::parse(&source)?;

    // Extract config.
    let mut config = RenderConfig::default();
    for item in &program.items {
        if let TopLevelItem::Config(cfg) = item {
            config = RenderConfig::from_config_block(cfg);
        }
    }

    // Apply CLI overrides.
    if let Some(fps) = fps_override {
        config.fps = fps;
    }
    if let Some(w) = width_override {
        config.width = w;
    }
    if let Some(h) = height_override {
        config.height = h;
    }

    // Load assets.
    let mut assets = AssetRegistry::new();
    let imports: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevelItem::Import(imp) = item {
                Some(imp.clone())
            } else {
                None
            }
        })
        .collect();
    assets.load_imports(&imports, &base_dir)?;

    // Extract custom pose definitions.
    let mut custom_poses: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for item in &program.items {
        if let TopLevelItem::PoseDef(pose_def) = item {
            let fields: Vec<(String, f64)> = pose_def
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.value))
                .collect();
            custom_poses.insert(pose_def.name.clone(), fields);
        }
    }

    // Process each scene.
    let scenes: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevelItem::Scene(scene) = item {
                Some(scene)
            } else {
                None
            }
        })
        .collect();

    if scenes.is_empty() {
        return Err(AnimError::Scene("no scenes found in source file".into()).into());
    }

    // If PNG output is requested, use the traditional approach (collect all frames).
    if let Some(dir) = png_dir {
        let mut all_frames = Vec::new();

        for scene_decl in &scenes {
            log::info!("Processing scene: {}", scene_decl.name);

            let resolved = resolve_scene(scene_decl, &assets)?;
            let compiled_timeline = timeline::compile(&resolved)?;

            // Check for character overlaps before rendering.
            let character_names: Vec<String> = resolved
                .entities
                .iter()
                .filter(|(_, e)| e.kind == EntityKind::Character)
                .map(|(name, _)| name.clone())
                .collect();
            timeline::check_overlaps(&compiled_timeline, &resolved.entities, &character_names)?;

            let frames = renderer::render_scene(
                &config,
                &compiled_timeline,
                &resolved.entities,
                resolved.set_name.as_deref(),
                &assets,
                &custom_poses,
            )?;
            all_frames.extend(frames);
        }

        video::encode_png_sequence(&all_frames, dir)?;

        println!(
            "Rendered {} scene(s), {} frames -> PNG sequence in {}",
            scenes.len(),
            all_frames.len(),
            dir.display(),
        );
        return Ok(());
    }

    // Otherwise, use streaming encoding to avoid storing all frames in memory.
    let mut total_duration = 0.0;
    let mut compiled_scenes = Vec::new();

    for scene_decl in &scenes {
        log::info!("Processing scene: {}", scene_decl.name);

        let resolved = resolve_scene(scene_decl, &assets)?;
        let compiled_timeline = timeline::compile(&resolved)?;

        let character_names: Vec<String> = resolved
            .entities
            .iter()
            .filter(|(_, e)| e.kind == EntityKind::Character)
            .map(|(name, _)| name.clone())
            .collect();
        timeline::check_overlaps(&compiled_timeline, &resolved.entities, &character_names)?;

        total_duration += compiled_timeline.duration;
        compiled_scenes.push((compiled_timeline, resolved));
    }

    let total_frames = (total_duration * config.fps as f64).ceil() as usize;
    log::info!(
        "Total: {} scenes, {} frames, {:.1}s, format: {:?}",
        scenes.len(),
        total_frames,
        total_duration,
        format
    );

    // For GIF format, we need to use the format-specific streaming encoder.
    // For other formats, use the standard streaming encoder.
    let use_parallel = !sequential && total_frames > 200;

    if use_parallel {
        log::info!(
            "Using parallel rendering with {} cores",
            renderer::parallel::num_cores()
        );
    } else {
        log::info!("Using sequential rendering");
    }

    // Use format-specific streaming encoder.
    let mut encoder = video::StreamingFormatEncoder::new(
        output,
        config.width,
        config.height,
        config.fps,
        format,
    )?;

    let mut frame_count = 0;

    for (compiled_timeline, resolved) in compiled_scenes {
        log::info!("Rendering scene: {}", resolved.name);

        let scene_frames = if use_parallel {
            let frames = renderer::parallel::render_scene_parallel(
                &config,
                &compiled_timeline,
                &resolved.entities,
                resolved.set_name.as_deref(),
                &assets,
                &custom_poses,
            )?;

            for frame in &frames {
                encoder.write_frame(frame)?;
                frame_count += 1;
            }
            frames.len()
        } else {
            // Use streaming renderer with callback.
            renderer::stream::render_scene_stream(
                &config,
                &compiled_timeline,
                &resolved.entities,
                resolved.set_name.as_deref(),
                &assets,
                &custom_poses,
                |frame| {
                    encoder.write_frame(frame)?;
                    frame_count += 1;
                    Ok(())
                },
            )?
        };

        log::info!("Scene complete: {} frames", scene_frames);
    }

    let frames_written = encoder.finish()?;

    println!(
        "Rendered {} scene(s), {} frames -> {} ({:?})",
        scenes.len(),
        frames_written,
        output.display(),
        format
    );

    Ok(())
}

fn cmd_check(input: &Path) -> Result<()> {
    let source = std::fs::read_to_string(input)?;
    let base_dir = input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let program = animdsl::parser::parse(&source)?;

    let mut n_imports = 0;
    let mut n_scenes = 0;
    let mut has_config = false;

    for item in &program.items {
        match item {
            TopLevelItem::Import(_) => n_imports += 1,
            TopLevelItem::Config(_) => has_config = true,
            TopLevelItem::Scene(_) => n_scenes += 1,
            TopLevelItem::PoseDef(_) => {}
        }
    }

    let mut assets = AssetRegistry::new();
    let imports: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevelItem::Import(imp) = item {
                Some(imp.clone())
            } else {
                None
            }
        })
        .collect();
    assets.load_imports(&imports, &base_dir)?;

    let scenes: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevelItem::Scene(scene) = item {
                Some(scene)
            } else {
                None
            }
        })
        .collect();

    for scene_decl in &scenes {
        let resolved = resolve_scene(scene_decl, &assets)?;
        let compiled_timeline = timeline::compile(&resolved)?;

        let character_names: Vec<String> = resolved
            .entities
            .iter()
            .filter(|(_, e)| e.kind == EntityKind::Character)
            .map(|(name, _)| name.clone())
            .collect();
        timeline::check_overlaps(&compiled_timeline, &resolved.entities, &character_names)?;
    }

    println!("OK: {}", input.display());
    println!("  Imports: {n_imports}");
    println!("  Config:  {}", if has_config { "yes" } else { "no" });
    println!("  Scenes:  {n_scenes}");
    println!("  Overlaps: none detected");

    Ok(())
}

fn cmd_dump(input: &Path) -> Result<()> {
    let source = std::fs::read_to_string(input)?;
    let program = animdsl::parser::parse(&source)?;
    let json = serde_json::to_string_pretty(&program)?;
    println!("{json}");
    Ok(())
}
