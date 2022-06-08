const WEIGHT_SATURATION: f32 = 0.24;
const WEIGHT_LUMA: f32 = 0.52;
const WEIGHT_POPULATION: f32 = 0.24;

const MIN_VIBRANT_SATURATION: f32 = 0.35;
const TARGET_VIBRANT_SATURATION: f32 = 1.0;

const TARGET_MUTED_SATURATION: f32 = 1.0;
const MAX_MUTED_SATURATION: f32 = 0.4;

const MIN_LIGHT_LUMA: f32 = 0.55;
const TARGET_LIGHT_LUMA: f32 = 0.74;

const TARGET_DARK_LUMA: f32 = 0.26;
const MAX_DARK_LUMA: f32 = 0.45;

const MIN_NORMAL_LUMA: f32 = 0.3;
const TARGET_NORMAL_LUMA: f32 = 0.5;
const MAX_NORMAL_LUMA: f32 = 0.7;

pub const LIGHT_VIBRANT: Target = Target {
    saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
    lightness_targets: (MIN_LIGHT_LUMA, TARGET_LIGHT_LUMA, 1.0),
    ..Target::new()
};

pub const VIBRANT: Target = Target {
    saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
    lightness_targets: (MIN_NORMAL_LUMA, TARGET_NORMAL_LUMA, MAX_NORMAL_LUMA),
    ..Target::new()
};

pub const DARK_VIBRANT: Target = Target {
    saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
    lightness_targets: (0.0, TARGET_DARK_LUMA, MAX_DARK_LUMA),
    ..Target::new()
};

pub const LIGHT_MUTED: Target = Target {
    saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
    lightness_targets: (MIN_LIGHT_LUMA, TARGET_LIGHT_LUMA, 1.0),
    ..Target::new()
};

pub const MUTED: Target = Target {
    saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
    lightness_targets: (MIN_NORMAL_LUMA, TARGET_NORMAL_LUMA, MAX_NORMAL_LUMA),
    ..Target::new()
};

pub const DARK_MUTED: Target = Target {
    saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
    lightness_targets: (0.0, TARGET_DARK_LUMA, MAX_DARK_LUMA),
    ..Target::new()
};

pub const DEFAULT_TARGETS: [Target; 6] = [LIGHT_VIBRANT, VIBRANT, DARK_VIBRANT, LIGHT_MUTED, MUTED, DARK_MUTED];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    // min, target, max
    saturation_targets: (f32, f32, f32),
    // min, target, max
    lightness_targets: (f32, f32, f32),
    // sat, luma, pop
    weights: (f32, f32, f32),
    is_exclusive: bool,
}

impl Default for Target {
    fn default() -> Self {
        Self::new()
    }
}

impl Target {
    pub const fn new() -> Self {
        Self {
            saturation_targets: (0.0, 0.5, 1.0),
            lightness_targets: (0.0, 0.5, 1.0),
            weights: (WEIGHT_SATURATION, WEIGHT_LUMA, WEIGHT_POPULATION),
            is_exclusive: true,
        }
    }

    pub(crate) fn normalize_weights(&mut self) {
        let weights_sum = self.weights.0.clamp(0.0, f32::MAX)
            + self.weights.1.clamp(0.0, f32::MAX)
            + self.weights.2.clamp(0.0, f32::MAX);

        if weights_sum != 0.0 {
            if self.weights.0 > 0.0 {
                self.weights.0 /= weights_sum;
            }

            if self.weights.1 > 0.0 {
                self.weights.1 /= weights_sum;
            }

            if self.weights.2 > 0.0 {
                self.weights.2 /= weights_sum;
            }
        }
    }

    pub fn minimum_saturation(self) -> f32 {
        self.saturation_targets.0
    }

    pub fn target_saturation(self) -> f32 {
        self.saturation_targets.1
    }

    pub fn maximum_saturation(self) -> f32 {
        self.saturation_targets.2
    }

    pub fn minimum_lightness(self) -> f32 {
        self.lightness_targets.0
    }

    pub fn target_lightness(self) -> f32 {
        self.lightness_targets.1
    }

    pub fn maximum_lightness(self) -> f32 {
        self.lightness_targets.2
    }

    pub fn saturation_weight(self) -> f32 {
        self.weights.0
    }

    pub fn lightness_weight(self) -> f32 {
        self.weights.1
    }

    pub fn population_weight(self) -> f32 {
        self.weights.2
    }

    pub fn is_exclusive(self) -> bool {
        self.is_exclusive
    }
}
