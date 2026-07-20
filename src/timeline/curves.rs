//! Animation curves — bezier path following for smooth motion.
//!
//! Supports cubic bezier curves for defining motion paths, waypoints,
//! and path-following mode for entities.

use crate::ast::Easing;
use crate::errors::AnimError;

/// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A cubic Bezier curve defined by four control points.
#[derive(Debug, Clone)]
pub struct BezierCurve {
    pub p0: Point2D,
    pub p1: Point2D,
    pub p2: Point2D,
    pub p3: Point2D,
}

impl BezierCurve {
    /// Create a new cubic Bezier curve.
    pub fn new(p0: Point2D, p1: Point2D, p2: Point2D, p3: Point2D) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Create a linear curve between two points.
    pub fn linear(p0: Point2D, p3: Point2D) -> Self {
        Self {
            p0,
            p1: p0.lerp(&p3, 0.33),
            p2: p0.lerp(&p3, 0.67),
            p3,
        }
    }

    /// Evaluate the curve at parameter t (0.0 to 1.0).
    pub fn evaluate(&self, t: f64) -> Point2D {
        let t = t.clamp(0.0, 1.0);
        let u = 1.0 - t;

        // Bernstein polynomial basis.
        let b0 = u * u * u;
        let b1 = 3.0 * u * u * t;
        let b2 = 3.0 * u * t * t;
        let b3 = t * t * t;

        Point2D {
            x: b0 * self.p0.x + b1 * self.p1.x + b2 * self.p2.x + b3 * self.p3.x,
            y: b0 * self.p0.y + b1 * self.p1.y + b2 * self.p2.y + b3 * self.p3.y,
        }
    }

    /// Compute the derivative (tangent) at parameter t.
    pub fn derivative(&self, t: f64) -> Point2D {
        let t = t.clamp(0.0, 1.0);
        let u = 1.0 - t;

        // Derivative of Bernstein basis.
        let d0 = -3.0 * u * u;
        let d1 = 3.0 * u * (u - 2.0 * t);
        let d2 = 3.0 * t * (2.0 * u - t);
        let d3 = 3.0 * t * t;

        Point2D {
            x: d0 * self.p0.x + d1 * self.p1.x + d2 * self.p2.x + d3 * self.p3.x,
            y: d0 * self.p0.y + d1 * self.p1.y + d2 * self.p2.y + d3 * self.p3.y,
        }
    }

    /// Compute the arc length of the curve.
    pub fn arc_length(&self, num_segments: usize) -> f64 {
        let mut length = 0.0;
        let mut prev = self.evaluate(0.0);

        for i in 1..=num_segments {
            let t = i as f64 / num_segments as f64;
            let current = self.evaluate(t);
            length += prev.distance(&current);
            prev = current;
        }

        length
    }

    /// Sample the curve at evenly spaced arc length intervals.
    pub fn sample_by_arc_length(&self, num_samples: usize, segments: usize) -> Vec<Point2D> {
        if num_samples == 0 {
            return Vec::new();
        }

        let total_length = self.arc_length(segments);
        let step = total_length / num_samples as f64;

        let mut samples = Vec::with_capacity(num_samples);
        let mut current_length = 0.0;
        let mut prev_t = 0.0;
        let mut prev_point = self.evaluate(0.0);

        samples.push(prev_point);

        for i in 1..num_samples {
            let target_length = i as f64 * step;

            // Binary search for t that gives the target arc length.
            let mut lo = prev_t;
            let mut hi = 1.0;
            let mut t = prev_t;

            for _ in 0..10 {
                let mid = (lo + hi) / 2.0;
                let mid_point = self.evaluate(mid);
                let mid_length = prev_point.distance(&mid_point)
                    + (mid - prev_t).abs() * self.arc_length(segments);

                if mid_length < target_length {
                    lo = mid;
                } else {
                    hi = mid;
                }
                t = mid;
            }

            let point = self.evaluate(t);
            samples.push(point);
            prev_t = t;
            prev_point = point;
        }

        samples
    }

    /// Get the start point.
    pub fn start(&self) -> Point2D {
        self.p0
    }

    /// Get the end point.
    pub fn end(&self) -> Point2D {
        self.p3
    }
}

/// A motion path composed of multiple Bezier curves (waypoints).
#[derive(Debug, Clone)]
pub struct MotionPath {
    pub segments: Vec<BezierCurve>,
    pub looped: bool,
}

impl MotionPath {
    /// Create a new motion path from waypoints.
    pub fn from_waypoints(waypoints: &[Point2D]) -> Self {
        if waypoints.len() < 2 {
            return Self {
                segments: Vec::new(),
                looped: false,
            };
        }

        let mut segments = Vec::with_capacity(waypoints.len() - 1);
        for i in 0..waypoints.len() - 1 {
            let p0 = waypoints[i];
            let p3 = waypoints[i + 1];
            // Generate control points for smooth transitions.
            let p1 = p0.lerp(&p3, 0.33);
            let p2 = p0.lerp(&p3, 0.67);
            segments.push(BezierCurve::new(p0, p1, p2, p3));
        }

        Self {
            segments,
            looped: false,
        }
    }

    /// Create a path with custom Bezier control points.
    pub fn from_bezier_segments(segments: Vec<BezierCurve>) -> Self {
        Self {
            segments,
            looped: false,
        }
    }

    /// Set whether the path loops back to the start.
    pub fn with_loop(mut self, looped: bool) -> Self {
        self.looped = looped;
        self
    }

    /// Evaluate the path at parameter t (0.0 to 1.0).
    pub fn evaluate(&self, t: f64) -> Option<Point2D> {
        if self.segments.is_empty() {
            return None;
        }

        let t = t.clamp(0.0, 1.0);
        let total_length = self.total_length();
        if total_length == 0.0 {
            return Some(self.segments[0].start());
        }

        // Find which segment we're in.
        let target_length = t * total_length;
        let mut accumulated = 0.0;

        for segment in &self.segments {
            let seg_length = segment.arc_length(20);
            if accumulated + seg_length >= target_length || segment == self.segments.last().unwrap() {
                let local_t = if seg_length > 0.0 {
                    (target_length - accumulated) / seg_length
                } else {
                    0.0
                };
                return Some(segment.evaluate(local_t.clamp(0.0, 1.0)));
            }
            accumulated += seg_length;
        }

        Some(self.segments.last().unwrap().end())
    }

    /// Get the tangent direction at parameter t.
    pub fn tangent(&self, t: f64) -> Option<Point2D> {
        if self.segments.is_empty() {
            return None;
        }

        let t = t.clamp(0.0, 1.0);
        let total_length = self.total_length();
        if total_length == 0.0 {
            return Some(Point2D::new(1.0, 0.0));
        }

        let target_length = t * total_length;
        let mut accumulated = 0.0;

        for segment in &self.segments {
            let seg_length = segment.arc_length(20);
            if accumulated + seg_length >= target_length || segment == self.segments.last().unwrap() {
                let local_t = if seg_length > 0.0 {
                    (target_length - accumulated) / seg_length
                } else {
                    0.0
                };
                let deriv = segment.derivative(local_t.clamp(0.0, 1.0));
                let len = (deriv.x * deriv.x + deriv.y * deriv.y).sqrt();
                if len > 0.0001 {
                    return Some(Point2D::new(deriv.x / len, deriv.y / len));
                }
                return Some(Point2D::new(1.0, 0.0));
            }
            accumulated += seg_length;
        }

        Some(Point2D::new(1.0, 0.0))
    }

    /// Get the total arc length of the path.
    pub fn total_length(&self) -> f64 {
        self.segments
            .iter()
            .map(|seg| seg.arc_length(20))
            .sum()
    }

    /// Get the start point of the path.
    pub fn start(&self) -> Option<Point2D> {
        self.segments.first().map(|seg| seg.start())
    }

    /// Get the end point of the path.
    pub fn end(&self) -> Option<Point2D> {
        self.segments.last().map(|seg| seg.end())
    }

    /// Sample the path at evenly spaced intervals.
    pub fn sample(&self, num_samples: usize) -> Vec<Point2D> {
        if self.segments.is_empty() || num_samples == 0 {
            return Vec::new();
        }

        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f64 / (num_samples - 1) as f64;
            if let Some(point) = self.evaluate(t) {
                samples.push(point);
            }
        }
        samples
    }
}

/// Keyframe with Bezier curve interpolation.
#[derive(Debug, Clone)]
pub struct BezierKeyframe {
    pub time: f64,
    pub value: f64,
    pub easing: Easing,
    /// Control points for custom bezier easing (x: time, y: value).
    pub control_points: Option<[Point2D; 2]>,
}

impl BezierKeyframe {
    /// Create a new keyframe with linear easing.
    pub fn new(time: f64, value: f64) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
            control_points: None,
        }
    }

    /// Create a keyframe with custom bezier easing.
    pub fn with_bezier(mut self, cp1: Point2D, cp2: Point2D) -> Self {
        self.control_points = Some([cp1, cp2]);
        self
    }

    /// Interpolate between two keyframes at time t.
    pub fn interpolate(&self, next: &Self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);

        // If we have custom bezier control points, use them for easing.
        if let (Some([cp1, cp2]), Some(_)) = (self.control_points.as_ref(), next.control_points.as_ref())
        {
            // Use the control points for easing interpolation.
            let eased_t = self.apply_bezier_easing(t, cp1, cp2);
            return self.value + (next.value - self.value) * eased_t;
        }

        // Otherwise, use the standard easing.
        let eased_t = self.apply_easing(t);
        self.value + (next.value - self.value) * eased_t
    }

    fn apply_easing(&self, t: f64) -> f64 {
        match self.easing {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }

    fn apply_bezier_easing(&self, t: f64, cp1: &Point2D, cp2: &Point2D) -> f64 {
        // Cubic bezier easing: x = t (time), y = value.
        let p0 = Point2D::new(0.0, 0.0);
        let p3 = Point2D::new(1.0, 1.0);
        let curve = BezierCurve::new(p0, *cp1, *cp2, p3);

        // Binary search for the x value.
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..20 {
            let mid = (lo + hi) / 2.0;
            let point = curve.evaluate(mid);
            if point.x < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let result = curve.evaluate((lo + hi) / 2.0);
        result.y.clamp(0.0, 1.0)
    }
}

/// Smooth a path using Catmull-Rom spline interpolation.
pub fn smooth_path(points: &[Point2D], num_segments: usize) -> Vec<Point2D> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut result = Vec::new();

    for i in 0..points.len() - 1 {
        let p0 = if i == 0 { points[0] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() {
            points[i + 2]
        } else {
            points[i + 1]
        };

        for j in 0..num_segments {
            let t = j as f64 / num_segments as f64;
            let point = catmull_rom_point(p0, p1, p2, p3, t);
            result.push(point);
        }
    }

    // Add the last point.
    if let Some(last) = points.last() {
        result.push(*last);
    }

    result
}

/// Catmull-Rom spline interpolation between p1 and p2.
fn catmull_rom_point(p0: Point2D, p1: Point2D, p2: Point2D, p3: Point2D, t: f64) -> Point2D {
    let t2 = t * t;
    let t3 = t2 * t;

    let x = 0.5 * (
        (2.0 * p1.x)
        + (-p0.x + p2.x) * t
        + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
        + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3
    );

    let y = 0.5 * (
        (2.0 * p1.y)
        + (-p0.y + p2.y) * t
        + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
        + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3
    );

    Point2D::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_curve_evaluation() {
        let p0 = Point2D::new(0.0, 0.0);
        let p3 = Point2D::new(1.0, 1.0);
        let curve = BezierCurve::linear(p0, p3);

        assert_eq!(curve.evaluate(0.0), p0);
        assert_eq!(curve.evaluate(1.0), p3);
        assert!(curve.evaluate(0.5).x > 0.3);
        assert!(curve.evaluate(0.5).y > 0.3);
    }

    #[test]
    fn test_motion_path() {
        let waypoints = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(0.5, 0.5),
            Point2D::new(1.0, 0.0),
        ];
        let path = MotionPath::from_waypoints(&waypoints);

        assert_eq!(path.start(), Some(Point2D::new(0.0, 0.0)));
        assert_eq!(path.end(), Some(Point2D::new(1.0, 0.0)));

        let mid = path.evaluate(0.5);
        assert!(mid.is_some());
        if let Some(p) = mid {
            assert!(p.x >= 0.0 && p.x <= 1.0);
            assert!(p.y >= -0.1 && p.y <= 0.6);
        }
    }

    #[test]
    fn test_bezier_keyframe_interpolation() {
        let kf1 = BezierKeyframe::new(0.0, 0.0);
        let kf2 = BezierKeyframe::new(1.0, 1.0);

        assert_eq!(kf1.interpolate(&kf2, 0.0), 0.0);
        assert_eq!(kf1.interpolate(&kf2, 1.0), 1.0);
        assert_eq!(kf1.interpolate(&kf2, 0.5), 0.5);
    }

    #[test]
    fn test_smooth_path() {
        let points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(0.5, 0.5),
            Point2D::new(1.0, 0.0),
        ];
        let smoothed = smooth_path(&points, 10);

        assert!(!smoothed.is_empty());
        assert_eq!(smoothed[0], points[0]);
        assert_eq!(*smoothed.last().unwrap(), points[points.len() - 1]);
    }
}
