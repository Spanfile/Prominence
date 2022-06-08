#[derive(Debug, Clone, Copy)]
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

    pub fn get_rgb(self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }

    pub fn get_hsl(self) -> (f32, f32, f32) {
        unimplemented!()
    }

    pub fn population(self) -> u32 {
        self.population
    }
}
