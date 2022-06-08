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
mod filter;
mod swatch;
mod target;

/// The default amount of colors to calculate at maximum while quantizing an image.
pub const DEFAULT_CALCULATE_NUMBER_COLORS: usize = 16;
/// The default area to resize the given image to before quantizing;
pub const DEFAULT_RESIZE_IMAGE_AREA: u32 = 112 * 112;

pub use crate::{swatch::Swatch, target::Target};
pub use image;

use crate::{
    color_cut_quantizer::ColorCutQuantizer,
    filter::{DefaultFilter, Filter},
};
use image::{math::Rect, GenericImageView, ImageBuffer};
use std::collections::{HashMap, HashSet};

/// A color palette derived from an image.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Palette {
    swatches: Vec<Swatch>,
    targets: Vec<Target>,
    selected_swatches: HashMap<u64, Option<Swatch>>,
}

/// A builder for a new [Palette].
pub struct PaletteBuilder<P>
where
    P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
{
    image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>,
    targets: Vec<Target>,
    maximum_color_count: usize,
    resize_area: Option<u32>,
    region: Option<Rect>,
    filters: Vec<Box<dyn Filter>>,
}

impl Palette {
    /// Return a new [`PaletteBuilder`] from a given image buffer.
    pub fn from_image<P>(image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>) -> PaletteBuilder<P>
    where
        P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
    {
        PaletteBuilder::from_image(image)
    }

    /// Returns the swatches in this palette.
    pub fn swatches(&self) -> &[Swatch] {
        &self.swatches
    }

    /// Returns the targets in this palette.
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// Returns the swatch corresponding to the preset light vibrant target, if it exists.
    pub fn light_vibrant_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::light_vibrant())
    }

    /// Returns the swatch corresponding to the preset vibrant target, if it exists.
    pub fn vibrant_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::vibrant())
    }

    /// Returns the swatch corresponding to the preset dark vibrant target, if it exists.
    pub fn dark_vibrant_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::dark_vibrant())
    }

    /// Returns the swatch corresponding to the preset light muted target, if it exists.
    pub fn light_muted_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::light_muted())
    }

    /// Returns the swatch corresponding to the preset muted target, if it exists.
    pub fn muted_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::muted())
    }

    /// Returns the swatch corresponding to the preset dark muted target, if it exists.
    pub fn dark_muted_swatch(&self) -> Option<Swatch> {
        self.get_swatch_for_target(Target::dark_muted())
    }

    /// Returns the color corresponding to the preset light vibrant target, if it exists.
    pub fn light_vibrant_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::light_vibrant())
            .map(|swatch| swatch.rgb())
    }

    /// Returns the color corresponding to the preset vibrant target, if it exists.
    pub fn vibrant_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::vibrant()).map(|swatch| swatch.rgb())
    }

    /// Returns the color corresponding to the preset dark vibrant target, if it exists.
    pub fn dark_vibrant_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::dark_vibrant())
            .map(|swatch| swatch.rgb())
    }

    /// Returns the color corresponding to the preset light muted target, if it exists.
    pub fn light_muted_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::light_muted())
            .map(|swatch| swatch.rgb())
    }

    /// Returns the color corresponding to the preset muted target, if it exists.
    pub fn muted_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::muted()).map(|swatch| swatch.rgb())
    }

    /// Returns the color corresponding to the preset dark vibrant target, if it exists.
    pub fn dark_muted_color(&self) -> Option<(u8, u8, u8)> {
        self.get_swatch_for_target(Target::dark_muted())
            .map(|swatch| swatch.rgb())
    }

    /// Returns the swatch corresponding to a given target, if it exists.
    pub fn get_swatch_for_target(&self, target: Target) -> Option<Swatch> {
        self.selected_swatches.get(&target.id()).copied().flatten()
    }

    /// Returns the most prominent color in the palette, which is the swatch with the largest population.
    pub fn most_prominent_color(&self) -> Option<(u8, u8, u8)> {
        self.swatches
            .iter()
            .max_by_key(|swatch| swatch.population())
            .map(|swatch| swatch.rgb())
    }
}

impl<P> PaletteBuilder<P>
where
    P: image::Pixel<Subpixel = u8> + 'static + std::cmp::Eq + std::hash::Hash,
{
    /// Returns a new [`PaletteBuilder`] from a given image buffer.
    pub fn from_image(image: ImageBuffer<P, Vec<<P as image::Pixel>::Subpixel>>) -> Self {
        Self {
            image,
            targets: Target::default_targets().to_vec(),
            maximum_color_count: DEFAULT_CALCULATE_NUMBER_COLORS,
            resize_area: Some(DEFAULT_RESIZE_IMAGE_AREA),
            region: None,
            filters: vec![Box::new(DefaultFilter)],
        }
    }

    pub fn from_swatches() -> Self {
        unimplemented!()
    }

    /// Set the desired area to shrink the image to before quantizing. Set to `None` to disable shrinking.
    ///
    /// By default the image will be shrunk to an area of 112 by 112 pixels, as defined in the
    /// [`DEFAULT_RESIZE_IMAGE_AREA`] constant. The image will not be grown if it is already smaller than the desired
    /// area.
    pub fn resize_image_area(self, resize_area: Option<u32>) -> Self {
        Self { resize_area, ..self }
    }

    /// Set a custom region to focus the palette generation on.
    ///
    /// The region is based on the original image. If the image is shrunk before quantizing (see
    /// [`PaletteBuilder::resize_image_area`]), the given region will be scaled accordingly to still cover a similar
    /// area in the shrunk image. By default, the entire image is used to generate the palette.
    pub fn region(self, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            region: Some(Rect { x, y, width, height }),
            ..self
        }
    }

    /// Add a custom target to the palette.
    ///
    /// By default, a set of preset targets are included in every palette. See [`Target::default_targets()`].
    pub fn add_target(mut self, target: Target) -> Self {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }

        self
    }

    /// Add a custom filter to the palette.
    ///
    /// A filter is used to reject certain colors from being included in the palette generation. By default, a filter
    /// that rejects colors very close to black and white, and colors close to the red side of the I line. TODO: what
    /// the fu- is the I line?
    pub fn add_filter<F>(mut self, filter: F) -> Self
    where
        F: Filter + 'static,
    {
        self.filters.push(Box::new(filter));
        self
    }

    /// Clears the set region.
    pub fn clear_region(self) -> Self {
        Self { region: None, ..self }
    }

    /// Removes all targets in the builder, including the presets.
    pub fn clear_targets(self) -> Self {
        Self {
            targets: Vec::new(),
            ..self
        }
    }

    /// Removes all filters in the builder, including the default filter.
    pub fn clear_filters(self) -> Self {
        Self {
            filters: Vec::new(),
            ..self
        }
    }

    /// Consume the builder and generate a new [`Palette`].
    pub fn generate(mut self) -> Palette {
        // scale down the image if requested
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

        // get pixels in the requested region, or in the entire image
        let pixels = if let Some(region) = self.region {
            self.image
                .view(region.x, region.y, region.width, region.height)
                .pixels()
                .map(|(_, _, p)| p)
                .collect()
        } else {
            self.image.pixels().copied().collect()
        };

        // quantize pixels, get swatches
        let quantizer = ColorCutQuantizer::new(pixels, self.maximum_color_count, self.filters);
        let swatches = quantizer.get_quantized_colors();

        // try to pick swatches for each target
        let mut used_colors = HashSet::new();
        let selected_swatches = self
            .targets
            .iter_mut()
            .map(|target| {
                target.normalize_weights();
                (
                    target.id(),
                    generate_scored_target(&swatches, *target, &mut used_colors),
                )
            })
            .collect();

        Palette {
            swatches,
            targets: self.targets,
            selected_swatches,
        }
    }

    fn scale_image_down(&mut self) -> bool
    where
        <P as image::Pixel>::Subpixel: 'static,
    {
        let (width, height) = self.image.dimensions();
        let area = width * height;

        let scale_ratio = match self.resize_area {
            Some(resize_area) if resize_area > 0 && area > resize_area => (resize_area as f32 / area as f32).sqrt(),
            _ => 0.0,
        };

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
        if let Some(max_scored_swatch) = get_max_scored_swatch_for_target(swatches, target, used_colors) {
            used_colors.insert(max_scored_swatch.rgb());
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
    let dominant_swatch = swatches.iter().copied().max_by_key(|swatch| swatch.population());

    swatches
        .iter()
        .copied()
        .filter(|swatch| should_be_scored_for_target(*swatch, target, used_colors))
        .max_by(|lhs, rhs| {
            generate_score(*lhs, dominant_swatch, target)
                .partial_cmp(&generate_score(*rhs, dominant_swatch, target))
                .unwrap()
        })
}

fn should_be_scored_for_target(swatch: Swatch, target: Target, used_colors: &HashSet<(u8, u8, u8)>) -> bool {
    let (_, saturation, lightness) = swatch.hsl();

    (target.minimum_saturation()..=target.maximum_saturation()).contains(&saturation)
        && (target.minimum_lightness()..=target.maximum_lightness()).contains(&lightness)
        && !used_colors.contains(&swatch.rgb())
}

fn generate_score(swatch: Swatch, dominant_swatch: Option<Swatch>, target: Target) -> f32 {
    let (_, saturation, lightness) = swatch.hsl();

    let max_population = if let Some(dominant_swatch) = dominant_swatch {
        dominant_swatch.population() as f32
    } else {
        1.0
    };

    // calculate scores for saturation and luminance based on their weight, and how close to the target
    // saturation/lightness the values are
    let saturation_score = target.saturation_weight() * (1.0 - (saturation - target.target_saturation()).abs());
    let lightness_score = target.lightness_weight() * (1.0 - (lightness - target.target_lightness()).abs());

    // calculate score for the population based on its weight and how large portion of the dominant population it is
    let population_score = target.population_weight() * (swatch.population() as f32 / max_population);

    saturation_score + lightness_score + population_score
}

fn rgb_to_hsl((r, g, b): (u8, u8, u8)) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let c = max - min;

    let l = (max + min) / 2.0;
    let (h, s) = if c == 0.0 {
        (0.0, 0.0)
    } else {
        let s = c / (1.0 - (2.0 * l - 1.0).abs());

        let (segment, shift) = if max == r {
            ((g - b) / c, if (g - b) / c < 0.0 { 360.0 / 60.0 } else { 0.0 })
        } else if max == g {
            ((b - r) / c, 120.0 / 60.0)
        } else {
            ((r - g) / c, 240.0 / 60.0)
        };

        (segment + shift, s)
    };

    (h * 60.0, s, l)
}
