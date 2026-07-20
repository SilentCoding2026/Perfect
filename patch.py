diff --git a/src/procedural/mod.rs b/src/procedural/mod.rs
--- a/src/procedural/mod.rs
+++ b/src/procedural/mod.rs
@@ -678,7 +678,7 @@
 fn draw_character(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     x: f64,
     y: f64,
@@ -897,7 +897,7 @@
 fn draw_torso(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     scale: f64,
     base_x: f64,
@@ -1191,7 +1191,7 @@
 fn draw_neck(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     base_x: f64,
     base_y: f64,
@@ -1212,7 +1212,7 @@
 fn draw_arm(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     base_x: f64,
     base_y: f64,
@@ -1199,7 +1199,7 @@
     r: f64,
     flip: f64,
-     _skin: [u8; 3],
+     skin: [u8; 3],
     opacity: f64,
 ) {
@@ -1225,7 +1225,7 @@
     state: &CharacterState,
     r: f64,
     flip: f64,
-     _skin: [u8; 3],
+     skin: [u8; 3],
     opacity: f64,
 ) {
@@ -1460,7 +1460,7 @@
 fn draw_leg(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     base_x: f64,
     base_y: f64,
@@ -1471,7 +1471,7 @@
     r: f64,
     width_factor: f64,
-     _flip: f64,
+     flip: f64,
     opacity: f64,
-     _skin: [u8; 3],
+     skin: [u8; 3],
 ) {
@@ -1652,7 +1652,7 @@
 fn draw_head(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     scale: f64,
     base_x: f64,
@@ -1660,7 +1660,7 @@
     head_tilt: f64,
     body_tilt: f64,
-     _flip: f64,
+     flip: f64,
     opacity: f64,
 ) {
@@ -2038,7 +2038,7 @@
 fn draw_eye(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     cx: f64,
     cy: f64,
@@ -2048,7 +2048,7 @@
     eye_angle: f64,
     eye_size: f64,
-     _flip: f64,
+     flip: f64,
     opacity: f64,
 ) {
@@ -2260,7 +2260,7 @@
 fn draw_mouth(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     cx: f64,
     cy: f64,
@@ -2463,7 +2463,7 @@
 fn draw_hair_back(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     base_x: f64,
     base_y: f64,
@@ -2471,7 +2471,7 @@
     head_r: f64,
     head_tilt: f64,
     opacity: f64,
-     _flip: f64,
+     flip: f64,
 ) {
@@ -2550,7 +2550,7 @@
 fn draw_hair_front(
     pixmap: &mut Pixmap,
-     _desc: &CharacterDesc,
+     desc: &CharacterDesc,
     state: &CharacterState,
     base_x: f64,
     base_y: f64,
@@ -2558,7 +2558,7 @@
     head_r: f64,
     head_tilt: f64,
     opacity: f64,
-     _flip: f64,
+     flip: f64,
 ) {
@@ -3272,7 +3272,7 @@
 ) -> Paint<'static> {
-     let paint = Paint::default();
+     let mut paint = Paint::default();
     paint.set_color(SkiaColor::from_rgba8(r, g, b, (opacity * 255.0) as u8));
     paint.anti_alias = true;
     paint
diff --git a/src/renderer/dof.rs b/src/renderer/dof.rs
--- a/src/renderer/dof.rs
+++ b/src/renderer/dof.rs
@@ -96,9 +96,9 @@
 fn apply_box_blur(pixmap: &mut Pixmap, radius: usize) {
-     let _w = pixmap.width() as usize;
-     let _h = pixmap.height() as usize;
-     let _data = pixmap.data_mut();
+     let w = pixmap.width() as usize;
+     let h = pixmap.height() as usize;
+     let data = pixmap.data_mut();
     
     if radius == 0 || w < radius * 2 || h < radius * 2 {
         return;
diff --git a/src/renderer/parallel.rs b/src/renderer/parallel.rs
--- a/src/renderer/parallel.rs
+++ b/src/renderer/parallel.rs
@@ -103,7 +103,7 @@
     if frames.is_empty() {
         return vec![];
     }
-     let mut result: Vec<Frame> = frames.into_iter().filter_map(|f| f).collect();
+     let result: Vec<Frame> = frames.into_iter().filter_map(|f| f).collect();
     
     // If there's only one frame, return it immediately
     if result.len() == 1 {