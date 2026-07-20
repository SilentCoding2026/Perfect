# AnimDSL Optimization Plan

## Overview
This document outlines all changes needed to make AnimDSL production-ready. The system works for short demos but requires optimization for longer animations and reliable rendering.

---

## 1. Performance Optimizations

### 1.1 Frame Streaming (HIGH PRIORITY)
**Issue:** Stores all frames in memory before encoding → memory overflow for long animations.

**Changes needed:**
- `src/video/mod.rs`: Add streaming encoder that writes frames to FFmpeg pipe
- `src/renderer/mod.rs`: Change `render_scene` to accept a callback instead of returning Vec<Frame>
- `src/main.rs`: Pipe frames directly to FFmpeg process

**Implementation:**
```rust
// New interface
pub fn render_scene_stream<F>(
    config: &RenderConfig,
    timeline: &Timeline,
    // ... other params
    mut frame_callback: F,
) -> Result<(), AnimError>
where
    F: FnMut(&Frame) -> Result<(), AnimError>;
```

### 1.2 Character Pose Caching (HIGH PRIORITY)
**Issue:** Re-renders same character pose every frame → CPU waste.

**Changes needed:**
- `src/procedural/mod.rs`: Add LRU cache for rendered character poses
- Cache key: `(CharacterDesc, PoseKey)` → rendered SVG/Pixmap
- Invalidate when pose changes

### 1.3 Parallel Frame Rendering (MEDIUM PRIORITY)
**Issue:** Renders frames sequentially on single core.

**Changes needed:**
- `src/renderer/mod.rs`: Use rayon for parallel frame rendering
- Each frame rendered independently
- Collect results and sort by frame index

### 1.4 Scene-Level Optimization (MEDIUM PRIORITY)
**Issue:** Renders full scene even when only small changes occur.

**Changes needed:**
- Detect static segments in timeline
- Reuse frames when nothing changes
- Only render new content during transitions/movement

---

## 2. Renderer Completion

### 2.1 Procedural Character Drawing (CRITICAL)
**Issue:** `src/procedural/mod.rs` is incomplete - missing full character rendering.

**Functions to implement:**
```rust
pub fn render_character(
    desc: &CharacterDesc,
    pose: &PoseValues,      // 23 fields
    skeleton: &SkeletonState,
    width: f32,
    height: f32,
) -> Pixmap;

// Missing sub-functions:
- draw_torso(body: &BodyDesc, pose: &PoseValues) -> Path
- draw_limb(endpoint1: Point, endpoint2: Point, width: f32, color: Color) -> Path
- draw_head(face: &FaceDesc, expression: &Expression) -> Path
- draw_hair(hair: &HairDesc, head_position: Point) -> Path
- draw_outfit(outfit: &OutfitDesc, pose: &PoseValues) -> Path
- draw_accessories(accessories: &[Accessory], pose: &PoseValues) -> Path
```

### 2.2 Bone/Skeleton Deformation (CRITICAL)
**Issue:** Skeleton interpolation exists but deformation not applied to character.

**Changes needed:**
- Apply skeleton bone transforms to each body part
- Deform meshes based on bone angles
- Implement skinning for smooth transitions

### 2.3 Expression Rendering (HIGH PRIORITY)
**Issue:** Face expressions defined but not rendered.

**Changes needed:**
- `draw_eyebrows(left: f64, right: f64, thickness: f64)`
- `draw_eyes(openness_left: f64, openness_right: f64, direction: f64)`
- `draw_mouth(smile: f64, open: f64, fullness: f64)`

---

## 3. Parsing & Validation

### 3.1 Better Error Messages (MEDIUM PRIORITY)
**Issue:** Parse errors are cryptic.

**Changes needed:**
- Add source location (line/column) to all errors
- Provide suggestions for common mistakes
- Validate position names and entity references

### 3.2 JSON Schema Validation (MEDIUM PRIORITY)
**Issue:** Character JSON files can have invalid values.

**Changes needed:**
- `src/assets/mod.rs`: Validate JSON fields and ranges
- Check enum values match allowed options
- Verify RGB values (0-255)

### 3.3 Timeline Validation (MEDIUM PRIORITY)
**Issue:** Actions can exceed scene duration.

**Changes needed:**
- Check total action durations vs scene duration
- Warn if scene duration is too short
- Auto-extend scene if actions exceed duration (optional)

---

## 4. Camera System

### 4.1 Camera Interpolation (HIGH PRIORITY)
**Issue:** Camera cuts instantly, no smooth transitions.

**Changes needed:**
- Implement smooth interpolation for `zoom-to`, `pan-to`
- Add acceleration/deceleration curves
- Support multiple camera targets simultaneously

### 4.2 Camera Bounds (MEDIUM PRIORITY)
**Issue:** Camera can zoom outside scene bounds.

**Changes needed:**
- Clamp zoom to prevent showing beyond scene edges
- Add padding limits for composition
- Implement "rule of thirds" framing

### 4.3 Depth of Field (LOW PRIORITY)
**Issue:** No depth effects.

**Changes needed:**
- Add blur based on z-distance
- Implement focus transitions

---

## 5. Timeline System

### 5.1 Easing Function Expansion (LOW PRIORITY)
**Issue:** Only 4 easing functions.

**Additional easings:**
- `ease-in-back` (overshoot)
- `ease-out-back` (overshoot)
- `ease-in-out-back` (overshoot)
- `bounce-in`, `bounce-out`, `bounce-in-out`
- `elastic-in`, `elastic-out`, `elastic-in-out`
- `steps(N)` (stepped animation)

### 5.2 Animation Curve Editor (LOW PRIORITY)
**Issue:** Can't define custom motion paths.

**Changes needed:**
- Support bezier curve paths for movement
- Define waypoints with timing
- Add path following mode

### 5.3 Audio Support (LOW PRIORITY)
**Issue:** No audio track support.

**Changes needed:**
- Add audio import statements
- Sync animation to audio timeline
- Export video with audio

---

## 6. Memory Management

### 6.1 Frame Compression (MEDIUM PRIORITY)
**Issue:** Raw RGBA frames use too much memory.

**Changes needed:**
- Store frames as compressed JPEG/PNG during rendering
- Decompress only when encoding
- Use memory-mapped files for large sequences

### 6.2 Asset Pooling (LOW PRIORITY)
**Issue:** SVG assets reloaded per frame.

**Changes needed:**
- Cache rendered SVG as Pixmap
- Reuse across frames when unchanged

---

## 7. CLI & UX

### 7.1 Progress Indicators (HIGH PRIORITY)
**Issue:** No feedback during rendering.

**Changes needed:**
- Add progress bar using `indicatif` crate
- Show estimated time remaining
- Display frame count and FPS

### 7.2 Render Modes (MEDIUM PRIORITY)
**Issue:** Only full render.

**Additional modes:**
- `--preview`: Render low-res proxy
- `--range START-END`: Render only specific frames
- `--skip N`: Skip every Nth frame for testing
- `--fast`: Disable some features for speed

### 7.3 Export Formats (MEDIUM PRIORITY)
**Issue:** Only MP4 and PNG sequence.

**Additional formats:**
- GIF (animated)
- WebM
- Image sequence (JPEG)
- QuickTime (MOV)

---

## 8. Testing

### 8.1 Unit Tests (HIGH PRIORITY)
**Issue:** Minimal test coverage.

**Tests needed:**
- Parser all statement types
- Timeline interpolation
- Character rendering output
- Overlap detection
- Camera operations

### 8.2 Integration Tests (MEDIUM PRIORITY)
**Tests needed:**
- Full scene rendering
- Asset loading
- Multi-scene composition
- Error conditions

### 8.3 Performance Benchmarks (LOW PRIORITY)
**Measure:**
- Frames per second rendering
- Memory usage
- Parse speed
- Export speed

---

## 9. Documentation

### 9.1 API Documentation (MEDIUM PRIORITY)
**Issue:** No Rust doc comments.

**Changes needed:**
- Document all public functions
- Add examples for each module
- Document internal architecture

### 9.2 Example Scripts (MEDIUM PRIORITY)
**Issue:** Only 5 examples.

**Additional examples:**
- Walk cycle demonstration
- Complex camera moves
- Character interactions
- Expression changes

### 9.3 Performance Guide (LOW PRIORITY)
**Content:**
- Optimizing scene complexity
- Reducing render time
- Memory best practices

---

## 10. Dependency Updates

### 10.1 Crate Updates (MEDIUM PRIORITY)
- `tiny-skia`: Update to latest
- `resvg`: Check for performance improvements
- `pest`: Update to latest grammar features
- `clap`: Update to v4 features

### 10.2 Optional Dependencies (LOW PRIORITY)
- Add `rayon` for parallelism
- Add `indicatif` for progress
- Add `image` for more format support
- Add `crossbeam` for channel communication

---

## Implementation Priority

### Phase 1 (Critical - Make it work):
1. Complete procedural character rendering (2.1, 2.2, 2.3)
2. Fix camera interpolation (4.1)
3. Add progress indicators (7.1)
4. Add unit tests (8.1)

### Phase 2 (Performance - Make it fast):
1. Frame streaming (1.1)
2. Character pose caching (1.2)
3. Parallel rendering (1.3)
4. Memory optimization (6.1)

### Phase 3 (Polish - Make it great):
1. Better error messages (3.1)
2. JSON validation (3.2)
3. Additional formats (7.3)
4. API documentation (9.1)

### Phase 4 (Nice to have):
1. Audio support (5.3)
2. Depth of field (4.3)
3. Animation curves (5.2)
4. Performance benchmarks (8.3)

---

## Estimated Effort

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1 | 4 critical tasks | 2-3 weeks |
| Phase 2 | 4 performance tasks | 1-2 weeks |
| Phase 3 | 4 polish tasks | 1 week |
| Phase 4 | 4 nice-to-haves | 1-2 weeks |

**Total:** 5-8 weeks for full completion

---

## Success Criteria

### Minimum Viable Product:
- [ ] Render 60-second animation without memory issues
- [ ] Smooth 30fps playback
- [ ] All character features rendered correctly
- [ ] Clear progress feedback
- [ ] No crashes on valid scripts

### Production Ready:
- [ ] 90% test coverage
- [ ] <1s load time for assets
- [ ] 15+ FPS rendering speed (1080p)
- [ ] All error messages actionable
- [ ] Full API documentation

---

## Notes for Implementation

### Maintain Architecture:
- Keep separation between parser, renderer, timeline
- Don't break existing DSL syntax
- Preserve backward compatibility

### Testing Strategy:
- Test each optimization independently
- Use benchmark tests to measure improvements
- Validate output quality visually

### Performance Targets:
- Memory: <2GB for 5-minute video
- Speed: >10 FPS rendering at 1080p
- CPU: Multi-core utilization
- GPU: Not required (pure CPU)

---

*This plan should be reviewed and prioritized based on project goals and available resources.*
