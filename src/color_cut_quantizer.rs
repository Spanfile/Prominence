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
        // begin by generating a histogram of quantized pixel values
        let mut hist = HashMap::new();
        for pixel in self.pixels.iter() {
            let pixel = pixel.map(|channel| modify_width(channel, 8, QUANTIZE_WORD_WIDTH) as u8);
            *hist.entry(pixel).or_insert(0) += 1;
        }

        // convert the histogram into a collection of (color, count) tuples, filtering out unwanted colors
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

        // the colors have to be ordered at this point, so order them by combining their channels into a single integer
        // where the red channel is the most signifcant and the blue the least
        colors.sort_by_key(|(pixel, _)| {
            let (r, g, b) = pixel_to_rgb(pixel);
            ((r as u32) << (QUANTIZE_WORD_WIDTH + QUANTIZE_WORD_WIDTH)) | ((g as u32) << QUANTIZE_WORD_WIDTH) | b as u32
        });

        if hist_len <= self.max_colors {
            // there are less colors than requested, no need for further processing; just return each color as a swatch
            colors
                .into_iter()
                .map(|(pixel, count)| Swatch::new(pixel_to_rgb(&pixel), count))
                .collect()
        } else {
            self.quantize_pixels(colors)
        }
    }

    fn quantize_pixels(self, mut colors: Vec<(P, u32)>) -> Vec<Swatch> {
        // create a priority queue of Vboxes with the first one containing all the given colors. Vbox comparison is
        // based on their volume, reversed, so the queue always pops the largest Vbox by volume first

        let mut pq = BinaryHeap::with_capacity(self.max_colors);
        pq.push(Vbox::new(&mut colors));

        // go through the queue until there are enough colors or no more boxes to split
        self.split_boxes(&mut pq);

        // return the remaining Vboxes converting them into swatches, filtering out unwanted colors
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

    fn should_ignore_color(&self, rgb: (u8, u8, u8)) -> bool {
        let hsl = crate::rgb_to_hsl(rgb);
        self.filters.iter().any(|filter| !filter.is_allowed(rgb, hsl))
    }

    fn split_boxes(&self, pq: &mut BinaryHeap<Vbox<'_, P>>) {
        while pq.len() < self.max_colors {
            if let Some(vbox) = pq.pop() {
                if vbox.can_split() {
                    // split the box in two and push them both back to the queue
                    let (left, right) = vbox.split_box();

                    pq.push(left);
                    pq.push(right);

                    continue;
                }
            }

            // if the queue is empty or the largest one cannot be split, there are no more Vboxes to split
            return;
        }
    }
}

impl<'a, P> Vbox<'a, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn new(colors: &'a mut [(P, u32)]) -> Self {
        // compute the boundaries of the Vbox to tightly fit around the colors within it

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
        // split the Vbox at the midpoint of its largest color dimension

        assert!(self.can_split());

        // sort the colors by the longest dimension so the midpoint can be searched for
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

        // keep a total sum of all the color populations and return the first one that crosses the midpoint. if no such
        // color is found, return the first index to still split the Vbox in two
        for (i, (_, count)) in self.colors.iter().enumerate() {
            pop += count;

            if pop >= midpoint {
                // in case the first color (index 0) already crosses the midpoint, return the color after it in order to
                // always split the Vbox in two
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
        // calculate the sum of all the color populations, as well as weighted sums of each color channel based on the
        // color populations
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

        // calculate the means of the channel weighted sums...
        let red_mean = red_sum as f32 / pop as f32;
        let green_mean = green_sum as f32 / pop as f32;
        let blue_mean = blue_sum as f32 / pop as f32;

        // ...and quantize them back into 8 bits
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
