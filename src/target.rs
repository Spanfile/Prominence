use std::hash::Hash;

const WEIGHT_SATURATION: f32 = 0.24;
const WEIGHT_LUMA: f32 = 0.52;
const WEIGHT_POPULATION: f32 = 0.24;

const MIN_VIBRANT_SATURATION: f32 = 0.35;
const TARGET_VIBRANT_SATURATION: f32 = 1.0;

const TARGET_MUTED_SATURATION: f32 = 0.3;
const MAX_MUTED_SATURATION: f32 = 0.4;

const MIN_LIGHT_LUMA: f32 = 0.55;
const TARGET_LIGHT_LUMA: f32 = 0.74;

const TARGET_DARK_LUMA: f32 = 0.26;
const MAX_DARK_LUMA: f32 = 0.45;

const MIN_NORMAL_LUMA: f32 = 0.3;
const TARGET_NORMAL_LUMA: f32 = 0.5;
const MAX_NORMAL_LUMA: f32 = 0.7;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Target {
    name: u64,
    // min, target, max
    saturation_targets: (f32, f32, f32),
    // min, target, max
    lightness_targets: (f32, f32, f32),
    // sat, luma, pop
    weights: (f32, f32, f32),
    is_exclusive: bool,
}

impl Target {
    pub fn default_targets() -> [Target; 6] {
        [
            Target::light_vibrant(),
            Target::vibrant(),
            Target::dark_vibrant(),
            Target::light_muted(),
            Target::muted(),
            Target::dark_muted(),
        ]
    }

    pub fn light_vibrant() -> Target {
        Target {
            name: 0,
            saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
            lightness_targets: (MIN_LIGHT_LUMA, TARGET_LIGHT_LUMA, 1.0),
            ..Target::new()
        }
    }

    pub fn vibrant() -> Target {
        Target {
            name: 1,
            saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
            lightness_targets: (MIN_NORMAL_LUMA, TARGET_NORMAL_LUMA, MAX_NORMAL_LUMA),
            ..Target::new()
        }
    }

    pub fn dark_vibrant() -> Target {
        Target {
            name: 2,
            saturation_targets: (MIN_VIBRANT_SATURATION, TARGET_VIBRANT_SATURATION, 1.0),
            lightness_targets: (0.0, TARGET_DARK_LUMA, MAX_DARK_LUMA),
            ..Target::new()
        }
    }

    pub fn light_muted() -> Target {
        Target {
            name: 3,
            saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
            lightness_targets: (MIN_LIGHT_LUMA, TARGET_LIGHT_LUMA, 1.0),
            ..Target::new()
        }
    }

    pub fn muted() -> Target {
        Target {
            name: 4,
            saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
            lightness_targets: (MIN_NORMAL_LUMA, TARGET_NORMAL_LUMA, MAX_NORMAL_LUMA),
            ..Target::new()
        }
    }

    pub fn dark_muted() -> Target {
        Target {
            name: 5,
            saturation_targets: (0.0, TARGET_MUTED_SATURATION, MAX_MUTED_SATURATION),
            lightness_targets: (0.0, TARGET_DARK_LUMA, MAX_DARK_LUMA),
            ..Target::new()
        }
    }

    pub fn new() -> Self {
        Self {
            name: rand::random(),
            saturation_targets: (0.0, 0.5, 1.0),
            lightness_targets: (0.0, 0.5, 1.0),
            weights: (WEIGHT_SATURATION, WEIGHT_LUMA, WEIGHT_POPULATION),
            is_exclusive: true,
        }
    }

    pub(crate) fn id(self) -> u64 {
        self.name
    }

    pub(crate) fn normalize_weights(&mut self) {
        let weights_sum = self.weights.0 + self.weights.1 + self.weights.2;

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

impl Default for Target {
    fn default() -> Self {
        Self::new()
    }
}

impl Eq for Target {}
impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Target {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
