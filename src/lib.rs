// Copyright 2022 Spanfile
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A library to extract prominent colors from an image.
//!
//! This library is a reimplementation of the Palette library in Android Jetpack. Android Jetpack is Copyright 2018 The
//! Android Open Source Project. Android Jetpack is licensed under the Apache License, Version 2.0.
//!
//! [Original source.](https://github.com/androidx/androidx/tree/f4eca2c46040cab36ebf7f34e68bdd973110e4a5/palette/palette/src/main/java/androidx/palette/graphics)
//!
//! [Android Jetpack license.](https://github.com/androidx/androidx/blob/7b7922489f9a7572f4462558691bf5550dd65c26/LICENSE.txt)

mod color_cut_quantizer;
mod swatch;
mod target;

pub const DEFAULT_CALCULATE_NUMBER_COLORS: usize = 16;
pub const DEFAULT_RESIZE_IMAGE_AREA: u32 = 112 * 112;

pub use crate::{swatch::Swatch, target::Target};
pub use image;
pub use palette;

use color_cut_quantizer::ColorCutQuantizer;
use image::{math::Rect, GenericImageView, ImageBuffer};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Palette {
    swatches: Vec<Swatch>,
    targets: Vec<Target>,
    selected_swatches: HashMap<Target, Option<Swatch>>,
}

#[derive(Debug)]
pub struct PaletteBuilder<P>
where
    P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
{
    image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>,
    targets: Vec<Target>,
    maximum_color_count: usize,
    resize_area: u32,
    region: Option<Rect>,
}

impl Palette {
    pub fn from_image<P, C>(image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>) -> PaletteBuilder<P>
    where
        P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
    {
        PaletteBuilder::from_image(image)
    }

    fn generate(swatches: Vec<Swatch>, mut targets: Vec<Target>) -> Palette {
        let mut selected_swatches = HashMap::new();
        let mut used_colors = HashSet::new();

        for target in &mut targets {
            target.normalize_weights();
            selected_swatches.insert(*target, generate_scored_target(&swatches, *target, &mut used_colors));
        }

        Self {
            swatches,
            targets,
            selected_swatches,
        }
    }
}

impl<P> PaletteBuilder<P>
where
    P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
{
    pub fn from_image(image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>) -> Self {
        Self {
            image,
            targets: target::DEFAULT_TARGETS.to_vec(),
            maximum_color_count: DEFAULT_CALCULATE_NUMBER_COLORS,
            resize_area: DEFAULT_RESIZE_IMAGE_AREA,
            region: None,
        }
    }

    pub fn from_swatches() -> Self {
        unimplemented!()
    }

    pub fn resize_image_area(self, resize_area: u32) -> Self {
        Self { resize_area, ..self }
    }

    pub fn region(self, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            region: Some(Rect { x, y, width, height }),
            ..self
        }
    }

    pub fn add_target(mut self, target: Target) -> Self {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }

        self
    }

    pub fn clear_region(self) -> Self {
        Self { region: None, ..self }
    }

    pub fn generate(mut self) -> Palette {
        if self.scale_image_down() {
            if let Some(mut region) = self.region {
                // scale down the region to match the new scaled image
                let scale = self.image.width() as f32 / self.image.height() as f32;

                region.x = (region.x as f32 * scale).floor() as u32;
                region.y = (region.y as f32 * scale).floor() as u32;
                region.width = ((region.width as f32 * scale) as u32 + region.x).min(self.image.width() - region.x);
                region.height = ((region.height as f32 * scale) as u32 + region.y).min(self.image.height() - region.y);

                self.region = Some(region);
            }
        }

        let view = if let Some(region) = self.region {
            self.image.view(region.x, region.y, region.width, region.height)
        } else {
            self.image.view(0, 0, self.image.width(), self.image.height())
        };

        let pixels = view.pixels().map(|(_, _, p)| p).collect();
        let quantizer = ColorCutQuantizer::new(pixels, self.maximum_color_count);
        let swatches = quantizer.get_quantized_colors();

        Palette::generate(swatches, self.targets)
    }

    fn scale_image_down(&mut self) -> bool
    where
        <P as image::Pixel>::Subpixel: 'static,
    {
        let mut scale_ratio = -1.0;
        let (width, height) = self.image.dimensions();

        if self.resize_area > 0 {
            let area = width * height;

            if area > self.resize_area {
                scale_ratio = (self.resize_area as f32 / area as f32).sqrt();
            }
        }

        if scale_ratio > 0.0 {
            self.image = image::imageops::resize(
                &self.image,
                (width as f32 * scale_ratio).ceil() as u32,
                (height as f32 * scale_ratio).ceil() as u32,
                image::imageops::FilterType::Nearest,
            );
            true
        } else {
            false
        }
    }
}

fn generate_scored_target(
    swatches: &[Swatch],
    target: Target,
    used_colors: &mut HashSet<(u8, u8, u8)>,
) -> Option<Swatch> {
    if target.is_exclusive() {
        if let Some(max_scored_swatch) = get_max_scored_swatch_for_target(swatches, target, &used_colors) {
            used_colors.insert(max_scored_swatch.get_rgb());
            return Some(max_scored_swatch);
        }
    }

    None
}

fn get_max_scored_swatch_for_target(
    swatches: &[Swatch],
    target: Target,
    used_colors: &HashSet<(u8, u8, u8)>,
) -> Option<Swatch> {
    let mut max_score = 0.0;
    let mut max_score_swatch = None;

    for swatch in swatches.iter().copied() {
        if should_be_scored_for_target(swatch, target, used_colors) {
            let score = generate_score(swatch, target);

            if max_score_swatch.is_none() || score > max_score {
                max_score_swatch = Some(swatch);
                max_score = score;
            }
        }
    }

    max_score_swatch
}

fn should_be_scored_for_target(swatch: Swatch, target: Target, used_colors: &HashSet<(u8, u8, u8)>) -> bool {
    let (_, saturation, lightness) = swatch.get_hsl();

    (target.minimum_saturation()..=target.maximum_saturation()).contains(&saturation)
        && (target.minimum_lightness()..=target.maximum_lightness()).contains(&lightness)
        && !used_colors.contains(&swatch.get_rgb())
}

fn generate_score(swatch: Swatch, target: Target) -> f32 {
    let (_, saturation, lightness) = swatch.get_hsl();
    let max_population = 1.0; // TODO: take from dominant swatch

    let saturation_score = if target.saturation_weight() > 0.0 {
        target.saturation_weight() * (1.0 - (saturation - target.target_saturation()).abs())
    } else {
        0.0
    };

    let luminance_score = if target.lightness_weight() > 0.0 {
        target.lightness_weight() * (1.0 - (lightness - target.target_lightness()).abs())
    } else {
        0.0
    };

    let population_score = if target.population_weight() > 0.0 {
        target.population_weight() * (swatch.population() as f32 / max_population)
    } else {
        0.0
    };

    saturation_score + luminance_score + population_score
}
