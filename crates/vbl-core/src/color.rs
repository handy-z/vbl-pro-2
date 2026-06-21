use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Option<Rgb> {
        let h = hex.trim().trim_start_matches('#');
        if h.len() != 6 {
            return None;
        }
        Some(Rgb::new(
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    Any,

    #[default]
    Majority,

    All,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Tolerance {
    pub per_channel: u8,
    #[serde(default)]
    pub aggregate: Aggregate,
}

impl Tolerance {
    pub fn exact() -> Self {
        Self {
            per_channel: 0,
            aggregate: Aggregate::All,
        }
    }

    pub fn separating(on: Rgb, off: Rgb) -> Tolerance {
        let d = channel_distance(on, off);
        let per_channel = if d <= 1 { 0 } else { ((d - 1) / 2).min(64) };
        Tolerance {
            per_channel,
            aggregate: Aggregate::All,
        }
    }

    pub fn matches(&self, sample: Rgb, target: Rgb) -> bool {
        let t = self.per_channel as i32;
        (sample.r as i32 - target.r as i32).abs() <= t
            && (sample.g as i32 - target.g as i32).abs() <= t
            && (sample.b as i32 - target.b as i32).abs() <= t
    }

    pub fn matches_region(&self, samples: &[Rgb], target: Rgb) -> bool {
        if samples.is_empty() {
            return false;
        }
        let hits = samples.iter().filter(|s| self.matches(**s, target)).count();
        match self.aggregate {
            Aggregate::Any => hits > 0,
            Aggregate::All => hits == samples.len(),
            Aggregate::Majority => hits * 2 > samples.len(),
        }
    }
}

fn channel_distance(a: Rgb, b: Rgb) -> u8 {
    a.r.abs_diff(b.r)
        .max(a.g.abs_diff(b.g))
        .max(a.b.abs_diff(b.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let t = Tolerance::exact();
        assert!(t.matches(Rgb::new(255, 255, 255), Rgb::new(255, 255, 255)));
        assert!(!t.matches(Rgb::new(254, 255, 255), Rgb::new(255, 255, 255)));
    }

    #[test]
    fn per_channel_tolerance() {
        let t = Tolerance {
            per_channel: 5,
            aggregate: Aggregate::All,
        };
        assert!(t.matches(Rgb::new(250, 255, 251), Rgb::new(255, 255, 255)));
        assert!(!t.matches(Rgb::new(249, 255, 255), Rgb::new(255, 255, 255)));
    }

    #[test]
    fn hex_parsing() {
        assert_eq!(Rgb::from_hex("#FFFFFF"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::from_hex("#ff0000"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(Rgb::from_hex("00FF80"), Some(Rgb::new(0, 255, 128)));
        assert_eq!(Rgb::from_hex("#fff"), None);
        assert_eq!(Rgb::from_hex("nothex!"), None);
    }

    #[test]
    fn separating_tolerance_includes_on_excludes_off() {
        let on = Rgb::new(255, 225, 148);
        let off = Rgb::new(40, 40, 40);
        let t = Tolerance::separating(on, off);
        assert!(t.matches(on, on)); // the on color always matches its own target
        assert!(!t.matches(off, on)); // the off color is excluded
    }

    #[test]
    fn separating_tolerance_clamps_and_handles_equal() {
        // Far-apart colors clamp to the 64 ceiling.
        assert_eq!(
            Tolerance::separating(Rgb::new(255, 255, 255), Rgb::new(0, 0, 0)).per_channel,
            64
        );
        // Identical / adjacent colors give an exact (0) tolerance.
        assert_eq!(
            Tolerance::separating(Rgb::new(10, 10, 10), Rgb::new(10, 10, 10)).per_channel,
            0
        );
    }

    #[test]
    fn region_aggregation() {
        let target = Rgb::new(0, 0, 0);
        let samples = [Rgb::new(0, 0, 0), Rgb::new(0, 0, 0), Rgb::new(40, 40, 40)];

        assert!(Tolerance {
            per_channel: 0,
            aggregate: Aggregate::Majority
        }
        .matches_region(&samples, target));
        assert!(Tolerance {
            per_channel: 0,
            aggregate: Aggregate::Any
        }
        .matches_region(&samples, target));
        assert!(!Tolerance {
            per_channel: 0,
            aggregate: Aggregate::All
        }
        .matches_region(&samples, target));
    }
}
