use crate::{filter::Filter, swatch::Swatch};
use std::collections::{BinaryHeap, HashMap};

const QUANTIZE_WORD_WIDTH: u32 = 5;
const QUANTIZE_WORD_MAX: u8 = (1 << QUANTIZE_WORD_WIDTH) - 1;

pub struct ColorCutQuantizer<P>
where
    P: image::Pixel<Subpixel = u8>,
{
    pixels: Vec<P>,
    max_colors: usize,
    filters: Vec<Box<dyn Filter>>,
}

struct Vbox<'a, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    colors: &'a mut [(P, u32)],
    population: u32,
    red_range: (u8, u8),
    green_range: (u8, u8),
    blue_range: (u8, u8),
}

enum Component {
    Red,
    Green,
    Blue,
}

impl<P> ColorCutQuantizer<P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    pub fn new(pixels: Vec<P>, max_colors: usize, filters: Vec<Box<dyn Filter>>) -> Self {
        Self {
            pixels,
            max_colors,
            filters,
        }
    }

    pub fn get_quantized_colors(self) -> Vec<Swatch> {
        let mut hist = HashMap::new();

        for pixel in self.pixels.iter() {
            let pixel = pixel.map(|channel| modify_width(channel, 8, QUANTIZE_WORD_WIDTH) as u8);
            *hist.entry(pixel).or_insert(0) += 1;
        }

        let hist_len = hist.len();
        let mut colors = hist
            .into_iter()
            .filter_map(|(pixel, count)| {
                if self.should_ignore_color(pixel_to_rgb(&pixel)) {
                    None
                } else {
                    Some((pixel, count))
                }
            })
            .collect::<Vec<_>>();

        colors.sort_by_key(|(pixel, _)| {
            let (r, g, b) = pixel_to_rgb(pixel);
            ((r as u32) << (QUANTIZE_WORD_WIDTH + QUANTIZE_WORD_WIDTH)) | ((g as u32) << QUANTIZE_WORD_WIDTH) | b as u32
        });

        if hist_len <= self.max_colors {
            colors
                .into_iter()
                .map(|(pixel, count)| Swatch::new(pixel_to_rgb(&pixel), count))
                .collect()
        } else {
            self.quantize_pixels(colors)
        }
    }

    fn should_ignore_color(&self, rgb: (u8, u8, u8)) -> bool {
        let hsl = crate::rgb_to_hsl(rgb);
        self.filters.iter().any(|filter| !filter.is_allowed(rgb, hsl))
    }

    fn quantize_pixels(self, mut colors: Vec<(P, u32)>) -> Vec<Swatch> {
        let mut pq = BinaryHeap::with_capacity(self.max_colors);
        pq.push(Vbox::new(&mut colors));

        self.split_boxes(&mut pq);
        pq.iter()
            .filter_map(|vbox| {
                let swatch = vbox.get_average_color();

                if !self.should_ignore_color(swatch.rgb()) {
                    Some(swatch)
                } else {
                    None
                }
            })
            .collect()
    }

    fn split_boxes(&self, pq: &mut BinaryHeap<Vbox<'_, P>>) {
        while pq.len() < self.max_colors {
            if let Some(vbox) = pq.pop() {
                if vbox.can_split() {
                    let (old, new) = vbox.split_box();

                    pq.push(old);
                    pq.push(new);

                    continue;
                }
            }

            return;
        }
    }
}

impl<'a, P> Vbox<'a, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn new(colors: &'a mut [(P, u32)]) -> Self {
        let mut population = 0;
        // min, max
        let (mut min_red, mut max_red) = (QUANTIZE_WORD_MAX, 0);
        let (mut min_green, mut max_green) = (QUANTIZE_WORD_MAX, 0);
        let (mut min_blue, mut max_blue) = (QUANTIZE_WORD_MAX, 0);

        for (pixel, count) in colors.iter() {
            let (r, g, b) = pixel_to_rgb(pixel);
            population += count;

            if r < min_red {
                min_red = r;
            }

            if r > max_red {
                max_red = r;
            }

            if g < min_green {
                min_green = g;
            }

            if g > max_green {
                max_green = g;
            }

            if b < min_blue {
                min_blue = b;
            }

            if b > max_blue {
                max_blue = b;
            }
        }

        Self {
            colors,
            population,
            red_range: (min_red, max_red),
            green_range: (min_green, max_green),
            blue_range: (min_blue, max_blue),
        }
    }

    fn volume(&self) -> u32 {
        (self.red_range.1 - self.red_range.0 + 1) as u32
            * (self.green_range.1 - self.green_range.0 + 1) as u32
            * (self.blue_range.1 - self.blue_range.0 + 1) as u32
    }

    fn split_box(mut self) -> (Vbox<'a, P>, Vbox<'a, P>) {
        assert!(self.can_split());

        self.sort_colors_by_longest_dimension();

        let split_point = self.find_split_point();
        let (old, new) = self.colors.split_at_mut(split_point);

        let old_box = Vbox::new(old);
        let new_box = Vbox::new(new);

        (old_box, new_box)
    }

    fn sort_colors_by_longest_dimension(&mut self) {
        let longest_dimension = self.get_longest_dimension();

        self.colors.sort_by(|(lhs, _), (rhs, _)| match longest_dimension {
            Component::Red => pixel_to_rgb(lhs).0.cmp(&pixel_to_rgb(rhs).0),
            Component::Green => pixel_to_rgb(lhs).1.cmp(&pixel_to_rgb(rhs).1),
            Component::Blue => pixel_to_rgb(lhs).2.cmp(&pixel_to_rgb(rhs).2),
        });
    }

    fn find_split_point(&mut self) -> usize {
        let midpoint = self.population / 2;
        let mut pop = 0;

        for (i, (_, count)) in self.colors.iter().enumerate() {
            pop += count;

            if pop >= midpoint {
                // never return the lowest index, so boxes will always be split in two
                return i.max(1);
            }
        }

        1
    }

    fn can_split(&self) -> bool {
        self.colors.len() > 1
    }

    fn get_longest_dimension(&self) -> Component {
        let red_length = self.red_range.1 - self.red_range.0;
        let green_length = self.green_range.1 - self.green_range.0;
        let blue_length = self.blue_range.1 - self.blue_range.0;

        if red_length >= green_length && red_length >= blue_length {
            Component::Red
        } else if green_length >= red_length && green_length >= blue_length {
            Component::Green
        } else {
            Component::Blue
        }
    }

    fn get_average_color(&self) -> Swatch {
        let (pop, red_sum, green_sum, blue_sum) =
            self.colors
                .iter()
                .fold((0, 0, 0, 0), |(pop, red_sum, green_sum, blue_sum), (pixel, count)| {
                    let (r, g, b) = pixel_to_rgb(pixel);

                    (
                        pop + count,
                        red_sum + r as u32 * count,
                        green_sum + g as u32 * count,
                        blue_sum + b as u32 * count,
                    )
                });

        let red_mean = red_sum as f32 / pop as f32;
        let green_mean = green_sum as f32 / pop as f32;
        let blue_mean = blue_sum as f32 / pop as f32;

        let red_quantized = modify_width(red_mean as u8, QUANTIZE_WORD_WIDTH, 8);
        let green_quantized = modify_width(green_mean as u8, QUANTIZE_WORD_WIDTH, 8);
        let blue_quantized = modify_width(blue_mean as u8, QUANTIZE_WORD_WIDTH, 8);

        Swatch::new((red_quantized, green_quantized, blue_quantized), pop)
    }
}

impl<P> Eq for Vbox<'_, P> where P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash {}
impl<P> PartialEq for Vbox<'_, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn eq(&self, other: &Self) -> bool {
        self.volume() == other.volume()
    }
}

impl<P> Ord for Vbox<'_, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.volume().cmp(&self.volume())
    }
}

impl<P> PartialOrd for Vbox<'_, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn pixel_to_rgb<P>(pixel: &P) -> (u8, u8, u8)
where
    P: image::Pixel<Subpixel = u8>,
{
    let rgb = pixel.to_rgb();
    (rgb.0[0], rgb.0[1], rgb.0[2])
}

fn modify_width(value: u8, current_width: u32, target_width: u32) -> u8 {
    if target_width > current_width {
        value.wrapping_shl(target_width - current_width)
    } else {
        value.wrapping_shr(current_width - target_width)
    }
}
