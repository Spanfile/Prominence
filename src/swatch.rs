#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Swatch {
    red: u8,
    blue: u8,
    green: u8,
    population: u32,
}

impl Swatch {
    pub fn new((red, green, blue): (u8, u8, u8), population: u32) -> Swatch {
        Self {
            red,
            blue,
            green,
            population,
        }
    }

    pub fn rgb(self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }

    pub fn hsl(self) -> (f32, f32, f32) {
        crate::rgb_to_hsl(self.rgb())
    }

    pub fn population(self) -> u32 {
        self.population
    }
}
