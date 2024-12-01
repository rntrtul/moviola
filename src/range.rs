#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

impl Default for Range {
    fn default() -> Self {
        Range {
            min: -1.0,
            max: 1.0,
            default: 0.0,
        }
    }
}

impl Range {
    pub fn new(min: f64, max: f64) -> Self {
        Range {
            min,
            max,
            default: ((max - min) / 2.0) + min,
        }
    }

    pub fn new_with_default(min: f64, max: f64, default: f64) -> Self {
        Range { min, max, default }
    }

    pub fn distance(&self) -> f64 {
        self.max - self.min
    }

    pub fn percent_from_value(&self, value: f64) -> f64 {
        (value - self.min) / self.distance()
    }

    pub fn value_from_percent(&self, percent: f64) -> f64 {
        (self.distance() * percent) + self.min
    }

    pub fn map_value_from_range(&self, range: Range, value: f64) -> f64 {
        self.value_from_percent(range.percent_from_value(value))
    }
}
