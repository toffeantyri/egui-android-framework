//! Constraints for layout, similar to Jetpack Compose.
//!
//! Parent passes constraints to children, children must respect them.

use egui::Vec2;

/// Constraints for layout — min/max width and height.
///
/// Similar to `Constraints` in Jetpack Compose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Minimum width in points.
    pub min_width: f32,
    /// Maximum width in points.
    pub max_width: f32,
    /// Minimum height in points.
    pub min_height: f32,
    /// Maximum height in points.
    pub max_height: f32,
}

impl Constraints {
    /// Create constraints with exact size (min == max).
    pub fn exact(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Create constraints with min/max ranges.
    pub fn ranged(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Create unconstrained constraints (0..INF).
    pub fn unconstrained() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }

    /// Clamp size to fit within constraints.
    pub fn clamp_size(&self, size: Vec2) -> Vec2 {
        Vec2::new(
            size.x.clamp(self.min_width, self.max_width),
            size.y.clamp(self.min_height, self.max_height),
        )
    }
}

impl Default for Constraints {
    fn default() -> Self {
        Self::unconstrained()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact() {
        let c = Constraints::exact(100.0, 200.0);
        assert_eq!(c.min_width, 100.0);
        assert_eq!(c.max_width, 100.0);
        assert_eq!(c.min_height, 200.0);
        assert_eq!(c.max_height, 200.0);
    }

    #[test]
    fn test_ranged() {
        let c = Constraints::ranged(10.0, 100.0, 20.0, 200.0);
        assert_eq!(c.min_width, 10.0);
        assert_eq!(c.max_width, 100.0);
        assert_eq!(c.min_height, 20.0);
        assert_eq!(c.max_height, 200.0);
    }

    #[test]
    fn test_unconstrained() {
        let c = Constraints::unconstrained();
        assert_eq!(c.min_width, 0.0);
        assert_eq!(c.max_width, f32::INFINITY);
        assert_eq!(c.min_height, 0.0);
        assert_eq!(c.max_height, f32::INFINITY);
    }

    #[test]
    fn test_default_is_unconstrained() {
        let c = Constraints::default();
        assert_eq!(c.min_width, 0.0);
        assert!(c.max_width.is_infinite());
    }

    #[test]
    fn test_clamp_size_below_min() {
        let c = Constraints::ranged(10.0, 100.0, 20.0, 200.0);
        let clamped = c.clamp_size(Vec2::new(5.0, 10.0));
        assert_eq!(clamped.x, 10.0);
        assert_eq!(clamped.y, 20.0);
    }

    #[test]
    fn test_clamp_size_above_max() {
        let c = Constraints::ranged(10.0, 100.0, 20.0, 200.0);
        let clamped = c.clamp_size(Vec2::new(200.0, 300.0));
        assert_eq!(clamped.x, 100.0);
        assert_eq!(clamped.y, 200.0);
    }

    #[test]
    fn test_clamp_size_within_range() {
        let c = Constraints::ranged(10.0, 100.0, 20.0, 200.0);
        let clamped = c.clamp_size(Vec2::new(50.0, 100.0));
        assert_eq!(clamped.x, 50.0);
        assert_eq!(clamped.y, 100.0);
    }

    #[test]
    fn test_exact_clamp() {
        let c = Constraints::exact(200.0, 100.0);
        let clamped = c.clamp_size(Vec2::new(300.0, 50.0));
        assert_eq!(clamped.x, 200.0);
        assert_eq!(clamped.y, 100.0);
    }
}
