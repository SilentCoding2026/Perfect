//! Procedural character drawing implementation.
//! Converts CharacterState into actual rendered pixels using tiny-skia.

use std::f64::consts::PI;
use tiny_skia::{
    Color as SkiaColor, FillRule, Paint, PathBuilder, Pixmap, Point, Stroke, Transform,
};

use super::*;

/// Render a character to a pixmap at the given position.
#[allow(clippy::too_many_arguments)]
pub fn render_character_to_pixmap(
    desc: &CharacterDesc,
    state: &CharacterState,
    pixmap: &mut Pixmap,
    x: f64,
    y: f64,
    scale_x: f64,
    scale_y: f64,
    rotation: f64,
    opacity: f64,
    width: u32,
    height: u32,
) -> Result<(), AnimError> {
    let cx = x * width as f64;
    let cy = y * height as f64;
    let base_size = height as f64 * 0.2; // Character height relative to canvas
    let scale = base_size * (0.7 + desc.body.height * 0.3);

    let mut transform = Transform::identity();
    transform = transform.pre_translate(cx, cy);
    transform = transform.pre_rotate(rotation);
    transform = transform.pre_scale(scale_x, scale_y);

    // Build character parts
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        desc.body.skin_color[0],
        desc.body.skin_color[1],
        desc.body.skin_color[2],
        (opacity * 255.0) as u8,
    );

    // Draw torso
    draw_torso(pixmap, &desc.body, state, &transform, scale, &paint);

    // Draw legs
    draw_legs(pixmap, &desc.body, state, &transform, scale);

    // Draw arms
    draw_arms(pixmap, &desc.body, state, &transform, scale);

    // Draw head
    draw_head(pixmap, &desc.face, state, &transform, scale);

    // Draw hair
    draw_hair(pixmap, &desc.hair, state, &transform, scale);

    // Draw outfit
    draw_outfit(pixmap, &desc.outfit, state, &transform, scale);

    // Draw accessories
    draw_accessories(pixmap, &desc.outfit.accessories, state, &transform, scale);

    Ok(())
}

fn draw_torso(
    pixmap: &mut Pixmap,
    body: &BodyDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
    paint: &Paint,
) {
    let mut path = PathBuilder::new();

    let w = 0.3 * scale;
    let h = 0.4 * scale;
    let squash = state.torso_squash;

    // Torso shape with line of action bend
    let bend = state.line_of_action * 0.15 * scale;

    path.move_to(
        -w * 0.5 + bend * 0.3,
        -h * 0.5 * squash + state.torso_bend * 0.2 * scale,
    );
    path.quad_to(
        bend,
        -h * 0.3 * squash,
        w * 0.5 + bend * 0.3,
        -h * 0.5 * squash,
    );
    path.quad_to(
        w * 0.6 + bend * 0.1,
        0.0,
        w * 0.5 - bend * 0.3,
        h * 0.5 * squash,
    );
    path.quad_to(
        -bend * 0.1,
        h * 0.6 * squash,
        -w * 0.5 - bend * 0.3,
        h * 0.5 * squash,
    );
    path.quad_to(
        -w * 0.6 - bend * 0.1,
        0.0,
        -w * 0.5 + bend * 0.3,
        -h * 0.5 * squash,
    );
    path.close();

    if let Some(path) = path.finish() {
        pixmap.fill_path(
            &path,
            paint,
            FillRule::Winding,
            *transform,
            None,
        );
    }
}

fn draw_legs(
    pixmap: &mut Pixmap,
    body: &BodyDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    let leg_len = 0.4 * scale;
    let leg_width = 0.08 * scale;

    let mut paint = Paint::default();
    paint.set_color_rgba8(
        body.skin_color[0],
        body.skin_color[1],
        body.skin_color[2],
        255,
    );

    for (side, angle, knee_bend) in [
        ("left", state.leg_left_angle, state.knee_left_bend),
        ("right", state.leg_right_angle, state.knee_right_bend),
    ] {
        let x_off = if side == "left" { -0.1 } else { 0.1 } * scale;
        let start_y = 0.25 * scale;

        // Upper leg
        let angle_rad = angle.to_radians();
        let upper_len = leg_len * 0.5;
        let knee_x = x_off + angle_rad.sin() * upper_len;
        let knee_y = start_y + angle_rad.cos() * upper_len;

        // Lower leg
        let lower_angle = angle + knee_bend * 45.0;
        let lower_rad = lower_angle.to_radians();
        let lower_len = leg_len * 0.5;
        let foot_x = knee_x + lower_rad.sin() * lower_len;
        let foot_y = knee_y + lower_rad.cos() * lower_len;

        let mut path = PathBuilder::new();
        path.move_to(x_off - leg_width * 0.5, start_y);
        path.line_to(x_off + leg_width * 0.5, start_y);
        path.line_to(knee_x + leg_width * 0.3, knee_y);
        path.line_to(knee_x - leg_width * 0.3, knee_y);
        path.line_to(foot_x, foot_y);
        path.line_to(foot_x - leg_width * 0.3, foot_y + leg_width * 0.3);
        path.close();

        if let Some(path) = path.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
        }
    }
}

fn draw_arms(
    pixmap: &mut Pixmap,
    body: &BodyDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    let arm_len = 0.35 * scale;
    let arm_width = 0.06 * scale;

    let mut paint = Paint::default();
    paint.set_color_rgba8(
        body.skin_color[0],
        body.skin_color[1],
        body.skin_color[2],
        255,
    );

    for (side, angle, elbow_bend, shoulder) in [
        ("left", state.arm_left_angle, state.elbow_left_bend, state.shoulder_left),
        ("right", state.arm_right_angle, state.elbow_right_bend, state.shoulder_right),
    ] {
        let x_off = if side == "left" { -0.25 } else { 0.25 } * scale;
        let start_y = -0.15 * scale + shoulder * 0.05 * scale;

        let angle_rad = angle.to_radians();
        let upper_len = arm_len * 0.5;
        let elbow_x = x_off + angle_rad.sin() * upper_len;
        let elbow_y = start_y + angle_rad.cos() * upper_len;

        let lower_angle = angle + elbow_bend * 90.0;
        let lower_rad = lower_angle.to_radians();
        let lower_len = arm_len * 0.5;
        let hand_x = elbow_x + lower_rad.sin() * lower_len;
        let hand_y = elbow_y + lower_rad.cos() * lower_len;

        let mut path = PathBuilder::new();
        path.move_to(x_off - arm_width * 0.5, start_y);
        path.line_to(x_off + arm_width * 0.5, start_y);
        path.line_to(elbow_x + arm_width * 0.4, elbow_y);
        path.line_to(elbow_x - arm_width * 0.4, elbow_y);
        path.line_to(hand_x, hand_y);
        path.line_to(hand_x - arm_width * 0.4, hand_y + arm_width * 0.3);
        path.close();

        if let Some(path) = path.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
        }
    }
}

fn draw_head(
    pixmap: &mut Pixmap,
    face: &FaceDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    let head_scale = scale * 0.2;
    let head_x = state.head_tilt * 0.02 * scale;
    let head_y = -0.38 * scale + state.head_nod * 0.01 * scale;

    let mut paint = Paint::default();
    paint.set_color_rgba8(
        face.eye_color[0],
        face.eye_color[1],
        face.eye_color[2],
        255,
    );

    // Head shape
    let mut path = PathBuilder::new();
    let r = head_scale * 0.8;
    path.push_circle(head_x, head_y, r);

    if let Some(path) = path.finish() {
        pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
    }

    // Eyes
    draw_eyes(pixmap, face, state, head_x, head_y, head_scale, transform);
    draw_eyebrows(pixmap, face, state, head_x, head_y, head_scale, transform);
    draw_mouth(pixmap, face, state, head_x, head_y, head_scale, transform);
}

fn draw_eyes(
    pixmap: &mut Pixmap,
    face: &FaceDesc,
    state: &CharacterState,
    hx: f64,
    hy: f64,
    hs: f64,
    transform: &Transform,
) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);

    let eye_spread = 0.25 * hs;
    let eye_size = hs * 0.15 * face.eye_size;

    for (side, open) in [
        ("left", state.expression.eye_open_left),
        ("right", state.expression.eye_open_right),
    ] {
        let ex = hx + if side == "left" { -eye_spread } else { eye_spread }
            + state.expression.eye_direction * 0.05 * hs;
        let ey = hy + 0.05 * hs;

        let eye_h = eye_size * open.min(1.0);

        let mut path = PathBuilder::new();
        path.push_rect(ex - eye_size, ey - eye_h * 0.5, eye_size * 2.0, eye_h);

        if let Some(path) = path.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
        }

        // Pupil
        let mut pupil = Paint::default();
        pupil.set_color_rgba8(
            face.eye_color[0],
            face.eye_color[1],
            face.eye_color[2],
            255,
        );
        let pupil_size = eye_size * 0.4;
        let mut ppath = PathBuilder::new();
        ppath.push_circle(
            ex + state.expression.eye_direction * 0.3 * eye_size,
            ey,
            pupil_size,
        );
        if let Some(ppath) = ppath.finish() {
            pixmap.fill_path(&ppath, &pupil, FillRule::Winding, *transform, None);
        }
    }
}

fn draw_eyebrows(
    pixmap: &mut Pixmap,
    face: &FaceDesc,
    state: &CharacterState,
    hx: f64,
    hy: f64,
    hs: f64,
    transform: &Transform,
) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(60, 40, 30, 255);

    let brow_width = 0.2 * hs;
    let brow_height = hs * 0.02 * (0.5 + face.eyebrow_thickness * 0.5);

    for (side, raise) in [
        ("left", state.expression.eyebrow_left),
        ("right", state.expression.eyebrow_right),
    ] {
        let bx = hx + if side == "left" { -0.2 } else { 0.2 } * hs;
        let by = hy - 0.2 * hs - raise * 0.08 * hs;

        let mut path = PathBuilder::new();
        path.move_to(bx - brow_width, by);
        path.line_to(bx + brow_width, by + brow_height * 0.3);
        path.line_to(bx + brow_width, by + brow_height);
        path.line_to(bx - brow_width, by + brow_height * 0.7);
        path.close();

        if let Some(path) = path.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
        }
    }
}

fn draw_mouth(
    pixmap: &mut Pixmap,
    face: &FaceDesc,
    state: &CharacterState,
    hx: f64,
    hy: f64,
    hs: f64,
    transform: &Transform,
) {
    let mut paint = Paint::default();
    let lip_red = (180.0 + 75.0 * (0.5 + face.lip_fullness * 0.5)) as u8;
    paint.set_color_rgba8(lip_red, 70, 70, 255);

    let mw = 0.15 * hs * (0.8 + state.expression.mouth_open * 0.4);
    let mh = hs * 0.04 * (0.5 + state.expression.mouth_open * 1.0);
    let smile = state.expression.mouth_smile * 0.05 * hs;

    let mut path = PathBuilder::new();
    path.move_to(hx - mw, hy + 0.15 * hs);
    path.quad_to(
        hx + smile,
        hy + 0.15 * hs - mh * 0.5,
        hx + mw,
        hy + 0.15 * hs + mh * 0.3,
    );
    path.quad_to(
        hx + smile * 0.5,
        hy + 0.15 * hs + mh,
        hx - mw,
        hy + 0.15 * hs + mh * 0.3,
    );
    path.close();

    if let Some(path) = path.finish() {
        pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
    }
}

fn draw_hair(
    pixmap: &mut Pixmap,
    hair: &HairDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(hair.color[0], hair.color[1], hair.color[2], 255);

    let hs = scale * 0.2;
    let hx = state.head_tilt * 0.02 * scale + state.hair_swing * 0.05 * scale;
    let hy = -0.38 * scale;

    match hair.style {
        HairStyle::Buzz => {
            // Very short: just a cap shape
            let mut path = PathBuilder::new();
            path.push_circle(hx, hy - 0.05 * hs, hs * 0.7);
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
            }
        }
        HairStyle::Short => {
            let mut path = PathBuilder::new();
            path.move_to(hx - hs * 0.7, hy - 0.1 * hs);
            path.quad_to(hx - hs * 0.4, hy - hs * 0.6, hx, hy - hs * 0.5);
            path.quad_to(hx + hs * 0.4, hy - hs * 0.6, hx + hs * 0.7, hy - 0.1 * hs);
            path.close();
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
            }
        }
        HairStyle::SlickedBack | HairStyle::Straight | HairStyle::Wavy => {
            let len = hs * (0.5 + hair.length * 0.5);
            let mut path = PathBuilder::new();
            path.move_to(hx - hs * 0.6, hy - 0.1 * hs);
            path.quad_to(hx - hs * 0.3, hy - hs * 0.6, hx, hy - hs * 0.5);
            path.quad_to(hx + hs * 0.3, hy - hs * 0.6, hx + hs * 0.6, hy - 0.1 * hs);
            path.quad_to(hx + hs * 0.5, hy + len * 0.3, hx + hs * 0.1, hy + len * 0.5);
            path.quad_to(hx, hy + len * 0.6, hx - hs * 0.1, hy + len * 0.5);
            path.quad_to(hx - hs * 0.5, hy + len * 0.3, hx - hs * 0.6, hy - 0.1 * hs);
            path.close();
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
            }
        }
        HairStyle::Messy => {
            let len = hs * (0.4 + hair.length * 0.4);
            for i in 0..5 {
                let angle = (i as f64 / 5.0 * PI * 1.5) + state.hair_swing * 0.3;
                let rx = hx + (angle).sin() * hs * 0.5;
                let ry = hy - hs * 0.3 + (angle).cos() * len * 0.5;
                let mut path = PathBuilder::new();
                path.move_to(rx - hs * 0.1, ry);
                path.quad_to(rx, ry - len * 0.3, rx + hs * 0.1, ry);
                path.quad_to(rx, ry + len * 0.2, rx - hs * 0.1, ry);
                path.close();
                if let Some(path) = path.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
                }
            }
        }
    }
}

fn draw_outfit(
    pixmap: &mut Pixmap,
    outfit: &OutfitDesc,
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(outfit.top.color[0], outfit.top.color[1], outfit.top.color[2], 255);

    // Simple outfit shapes overlaid on body
    match outfit.top.kind {
        ClothingKind::TShirt | ClothingKind::Hoodie => {
            let w = 0.35 * scale;
            let h = 0.25 * scale;
            let mut path = PathBuilder::new();
            path.move_to(-w, -0.2 * scale);
            path.quad_to(0.0, -0.35 * scale, w, -0.2 * scale);
            path.line_to(w, 0.05 * scale);
            path.line_to(-w, 0.05 * scale);
            path.close();
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
            }
        }
        ClothingKind::TrenchCoat | ClothingKind::Suit => {
            let w = 0.4 * scale;
            let h = 0.4 * scale;
            let mut path = PathBuilder::new();
            path.move_to(-w * 0.6, -0.15 * scale);
            path.quad_to(0.0, -0.3 * scale, w * 0.6, -0.15 * scale);
            path.line_to(w * 0.7, 0.1 * scale);
            path.line_to(w * 0.5, 0.35 * scale);
            path.line_to(-w * 0.5, 0.35 * scale);
            path.line_to(-w * 0.7, 0.1 * scale);
            path.close();
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
            }
        }
        _ => {}
    }

    // Bottom clothing
    let mut bottom_paint = Paint::default();
    bottom_paint.set_color_rgba8(
        outfit.bottom.color[0],
        outfit.bottom.color[1],
        outfit.bottom.color[2],
        255,
    );

    match outfit.bottom.kind {
        ClothingKind::Pants | ClothingKind::Jeans => {
            let w = 0.2 * scale;
            let mut path = PathBuilder::new();
            path.move_to(-w, 0.25 * scale);
            path.line_to(w, 0.25 * scale);
            path.line_to(w * 0.8, 0.5 * scale);
            path.line_to(-w * 0.8, 0.5 * scale);
            path.close();
            if let Some(path) = path.finish() {
                pixmap.fill_path(&path, &bottom_paint, FillRule::Winding, *transform, None);
            }
        }
        _ => {}
    }
}

fn draw_accessories(
    pixmap: &mut Pixmap,
    accessories: &[Accessory],
    state: &CharacterState,
    transform: &Transform,
    scale: f64,
) {
    for acc in accessories {
        let mut paint = Paint::default();
        paint.set_color_rgba8(acc.color[0], acc.color[1], acc.color[2], 255);

        match acc.kind {
            AccessoryKind::Hat | AccessoryKind::Fedora => {
                let hx = state.head_tilt * 0.02 * scale;
                let hy = -0.45 * scale;
                let hs = scale * 0.1;

                let mut path = PathBuilder::new();
                path.move_to(hx - hs * 1.2, hy - hs * 0.3);
                path.quad_to(hx, hy - hs * 0.8, hx + hs * 1.2, hy - hs * 0.3);
                path.line_to(hx + hs * 0.8, hy + hs * 0.1);
                path.line_to(hx - hs * 0.8, hy + hs * 0.1);
                path.close();

                if let Some(path) = path.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
                }

                if acc.kind == AccessoryKind::Fedora {
                    // Brim
                    let mut brim = Paint::default();
                    brim.set_color_rgba8(
                        (acc.color[0] as f64 * 0.8) as u8,
                        (acc.color[1] as f64 * 0.8) as u8,
                        (acc.color[2] as f64 * 0.8) as u8,
                        255,
                    );
                    let mut bpath = PathBuilder::new();
                    bpath.move_to(hx - hs * 1.4, hy - hs * 0.25);
                    bpath.quad_to(hx, hy - hs * 0.4, hx + hs * 1.4, hy - hs * 0.25);
                    if let Some(bpath) = bpath.finish() {
                        pixmap.stroke_path(
                            &bpath,
                            &brim,
                            &Stroke::default(),
                            *transform,
                            None,
                        );
                    }
                }
            }
            AccessoryKind::Glasses => {
                let hx = state.head_tilt * 0.02 * scale;
                let hy = -0.36 * scale;
                let gs = scale * 0.03;

                for (side, _) in [("left", true), ("right", false)] {
                    let gx = hx + if side == "left" { -0.2 } else { 0.2 } * scale;
                    let mut path = PathBuilder::new();
                    path.move_to(gx - gs * 2.0, hy - gs);
                    path.quad_to(gx, hy - gs * 2.0, gx + gs * 2.0, hy - gs);
                    path.quad_to(gx + gs * 2.0, hy + gs, gx + gs * 2.0, hy + gs);
                    path.quad_to(gx, hy + gs * 2.0, gx - gs * 2.0, hy + gs);
                    path.quad_to(gx - gs * 2.0, hy - gs, gx - gs * 2.0, hy - gs);
                    path.close();

                    if let Some(path) = path.finish() {
                        pixmap.stroke_path(
                            &path,
                            &paint,
                            &Stroke {
                                width: gs * 0.5,
                                ..Default::default()
                            },
                            *transform,
                            None,
                        );
                    }
                }
            }
            AccessoryKind::Tie => {
                let mut path = PathBuilder::new();
                path.move_to(-0.03 * scale, -0.15 * scale);
                path.quad_to(0.0, -0.1 * scale, 0.03 * scale, -0.15 * scale);
                path.line_to(0.02 * scale, 0.15 * scale);
                path.line_to(-0.02 * scale, 0.15 * scale);
                path.close();

                if let Some(path) = path.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
                }
            }
            AccessoryKind::Scarf => {
                let mut path = PathBuilder::new();
                path.move_to(-0.3 * scale, -0.2 * scale);
                path.quad_to(0.0, -0.25 * scale, 0.3 * scale, -0.2 * scale);
                path.line_to(0.25 * scale, -0.1 * scale);
                path.line_to(-0.25 * scale, -0.1 * scale);
                path.close();

                // Scarf tails
                for side in [-1.0, 1.0] {
                    path.move_to(side * 0.2 * scale, -0.1 * scale);
                    path.quad_to(side * 0.25 * scale, 0.1 * scale, side * 0.2 * scale, 0.2 * scale);
                }
                path.close();

                if let Some(path) = path.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
                }
            }
            AccessoryKind::Belt => {
                let mut path = PathBuilder::new();
                path.move_to(-0.2 * scale, 0.05 * scale);
                path.line_to(0.2 * scale, 0.05 * scale);
                path.line_to(0.2 * scale, 0.08 * scale);
                path.line_to(-0.2 * scale, 0.08 * scale);
                path.close();

                if let Some(path) = path.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, *transform, None);
                }
            }
        }
    }
}
