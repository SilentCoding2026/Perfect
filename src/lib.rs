//! # AnimDSL
//!
//! A domain-specific language for generating 2D animated movie scenes.
//!
//! This crate provides:
//! - A parser for the `.anim` DSL language
//! - A procedural character rendering engine
//! - A timeline system for animating characters and cameras
//! - Video encoding via FFmpeg
//! - Audio support with timeline synchronization
//! - Depth of field rendering
//! - Bezier animation curves
//!
//! ## Quick Start
//!
//! ```no_run
//! use animdsl::{parser, renderer, timeline, video};
//!
//! # fn example() -> Result<(), animdsl::errors::AnimError> {
//! // Parse a script
//! let source = std::fs::read_to_string("scene.anim")?;
//! let program = parser::parse(&source)?;
//!
//! // ... resolve scenes, compile timeline ...
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | `audio` | Audio track loading and timeline synchronization |
//! | `parser` | Parses `.anim` files into an AST |
//! | `ast` | Abstract Syntax Tree definitions |
//! | `scene` | Scene resolution and entity management |
//! | `timeline` | Timeline compilation and evaluation |
//! | `renderer` | Frame rendering with caching and parallelism |
//! | `procedural` | Procedural character drawing engine |
//! | `skeleton` | Bone/rig system for characters |
//! | `assets` | Asset loading and validation |
//! | `video` | Video encoding with multiple formats |
//! | `errors` | Error types and handling |

pub mod assets;
pub mod ast;
pub mod audio;
pub mod errors;
pub mod parser;
pub mod procedural;
pub mod renderer;
pub mod scene;
pub mod skeleton;
pub mod timeline;
pub mod video;

/// Re-export commonly used types.
pub use errors::AnimError;
