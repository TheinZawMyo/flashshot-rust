//! Shared geometry primitives.
//!
//! Both the GPUI overlay preview and the PNG exporter build their drawings from
//! the polygons produced here, so what you see on screen is what gets exported.

use std::ops::{Add, Div, Mul, Sub};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.len();
        if len < 1e-5 {
            Self::new(0.0, 0.0)
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    /// Rotated 90 degrees, used to offset a line into a quad.
    pub fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn distance_to(self, other: V2) -> f32 {
        (other - self).len()
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, rhs: Self) -> Self::Output {
        V2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, rhs: Self) -> Self::Output {
        V2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, rhs: f32) -> Self::Output {
        V2::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for V2 {
    type Output = V2;
    fn div(self, rhs: f32) -> Self::Output {
        V2::new(self.x / rhs, self.y / rhs)
    }
}

/// A rectangle in overlay (logical) coordinates.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn empty() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn from_points(a: V2, b: V2) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        Self::new(x, y, (a.x - b.x).abs(), (a.y - b.y).abs())
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    pub fn clamp_point(&self, p: V2) -> V2 {
        V2::new(
            p.x.clamp(self.x, self.right()),
            p.y.clamp(self.y, self.bottom()),
        )
    }

    /// Keep this rect inside `outer`, shrinking it when needed.
    pub fn clamp_within(&self, outer: RectF) -> RectF {
        let w = self.w.min(outer.w);
        let h = self.h.min(outer.h);
        let x = self.x.min(outer.right() - w).max(outer.x);
        let y = self.y.min(outer.bottom() - h).max(outer.y);
        RectF::new(x, y, w, h)
    }

    /// Polygon corners, clockwise.
    pub fn polygon(&self) -> Vec<V2> {
        vec![
            V2::new(self.x, self.y),
            V2::new(self.right(), self.y),
            V2::new(self.right(), self.bottom()),
            V2::new(self.x, self.bottom()),
        ]
    }
}

/// Turn a line into a filled quad of the given thickness.
pub fn thick_line(a: V2, b: V2, width: f32) -> Vec<V2> {
    let delta = b - a;
    if delta.len() < 0.01 {
        return square(a, width);
    }
    let offset = delta.normalized().perpendicular() * (width.max(1.0) / 2.0);
    vec![a + offset, b + offset, b - offset, a - offset]
}

fn square(center: V2, width: f32) -> Vec<V2> {
    let half = width.max(1.0) / 2.0;
    vec![
        V2::new(center.x - half, center.y - half),
        V2::new(center.x + half, center.y - half),
        V2::new(center.x + half, center.y + half),
        V2::new(center.x - half, center.y + half),
    ]
}

/// A shaft plus a triangular head.
pub fn arrow(a: V2, b: V2, width: f32) -> Vec<Vec<V2>> {
    let delta = b - a;
    if delta.len() < 1.0 {
        return vec![square(a, width)];
    }
    let dir = delta.normalized();
    let head_len = (width * 3.5).max(16.0).min(delta.len() * 0.9);
    let head_half = head_len * 0.45;
    let base = b - dir * head_len;
    let normal = dir.perpendicular() * head_half;

    let mut polygons = vec![thick_line(a, base, width)];
    polygons.push(vec![b, base + normal, base - normal]);
    polygons
}

/// Four quads forming the outline of a rectangle.
pub fn rect_outline(rect: RectF, width: f32) -> Vec<Vec<V2>> {
    let w = width.max(1.0);
    vec![
        thick_line(
            V2::new(rect.x, rect.y),
            V2::new(rect.right(), rect.y),
            w,
        ),
        thick_line(
            V2::new(rect.right(), rect.y),
            V2::new(rect.right(), rect.bottom()),
            w,
        ),
        thick_line(
            V2::new(rect.right(), rect.bottom()),
            V2::new(rect.x, rect.bottom()),
            w,
        ),
        thick_line(V2::new(rect.x, rect.bottom()), V2::new(rect.x, rect.y), w),
    ]
}

/// Freehand stroke: a quad per segment plus a joint at every interior point.
pub fn polyline(points: &[V2], width: f32) -> Vec<Vec<V2>> {
    let mut polygons = Vec::new();
    if points.is_empty() {
        return polygons;
    }
    if points.len() == 1 {
        polygons.push(square(points[0], width));
        return polygons;
    }
    for pair in points.windows(2) {
        polygons.push(thick_line(pair[0], pair[1], width));
    }
    for point in &points[1..points.len() - 1] {
        polygons.push(square(*point, width));
    }
    polygons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_reversed_points_is_normalized() {
        let rect = RectF::from_points(V2::new(10.0, 20.0), V2::new(4.0, 6.0));
        assert_eq!(rect, RectF::new(4.0, 6.0, 6.0, 14.0));
    }

    #[test]
    fn thick_line_has_four_corners() {
        let quad = thick_line(V2::new(0.0, 0.0), V2::new(10.0, 0.0), 4.0);
        assert_eq!(quad.len(), 4);
        assert_eq!(quad[0], V2::new(0.0, 2.0));
        assert_eq!(quad[2], V2::new(10.0, -2.0));
    }

    #[test]
    fn arrow_has_shaft_and_head() {
        let polygons = arrow(V2::new(0.0, 0.0), V2::new(100.0, 0.0), 4.0);
        assert_eq!(polygons.len(), 2);
        assert_eq!(polygons[1].len(), 3);
    }
}
