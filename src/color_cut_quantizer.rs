use crate::swatch::Swatch;
use std::collections::{BinaryHeap, HashMap};

const QUANTIZE_WORD_WIDTH: u8 = 5;
const QUANTIZE_WORD_MASK: u8 = (1 << QUANTIZE_WORD_WIDTH) - 1;

pub struct ColorCutQuantizer<P>
where
    P: image::Pixel<Subpixel = u8>,
{
    pixels: Vec<P>,
    max_colors: usize,
}

struct Vbox<'a, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    colors: &'a mut [(P, u32)],
    population: u32,
    red: (u8, u8),
    green: (u8, u8),
    blue: (u8, u8),
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
    pub fn new(pixels: Vec<P>, max_colors: usize) -> Self {
        Self { pixels, max_colors }
    }

    pub fn get_quantized_colors(mut self) -> Vec<Swatch> {
        let mut hist = HashMap::new();

        for pixel in &mut self.pixels {
            pixel.apply(|c| modify_word_with(c, 8, QUANTIZE_WORD_WIDTH));
            *hist.entry(*pixel).or_insert(0) += 1;
        }

        if hist.len() <= self.max_colors {
            hist.into_iter()
                .map(|(pixel, count)| Swatch::new(pixel_to_rgb(&pixel), count))
                .collect()
        } else {
            let mut colors = hist
                .into_iter()
                .map(|(pixel, count)| (pixel, count))
                .collect::<Vec<_>>();

            self.quantize_pixels(&mut colors)
        }
    }

    fn quantize_pixels(self, colors: &mut [(P, u32)]) -> Vec<Swatch> {
        let mut pq = BinaryHeap::with_capacity(self.max_colors);
        pq.push(Vbox::new(colors));

        split_boxes(&mut pq, self.max_colors);
        pq.into_iter().map(|vbox| vbox.get_average_color()).collect()
    }
}

impl<'a, P> Vbox<'a, P>
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    fn new(colors: &'a mut [(P, u32)]) -> Self {
        let mut population = 0;
        let mut red = (255, 0);
        let mut green = (255, 0);
        let mut blue = (255, 0);

        for (pixel, count) in colors.iter() {
            population += *count as u32;

            let quantized = pixel.map(|c| modify_word_with(c, 8, QUANTIZE_WORD_WIDTH));
            let rgb = quantized.to_rgb();
            let (r, g, b) = (rgb.0[0], rgb.0[1], rgb.0[2]);

            if r < red.0 {
                red.0 = r;
            }

            if r > red.1 {
                red.1 = r;
            }

            if g < green.0 {
                green.0 = g;
            }

            if g > green.1 {
                green.1 = g;
            }

            if b < blue.0 {
                blue.0 = b;
            }

            if b > blue.1 {
                blue.1 = b;
            }
        }

        Self {
            colors,
            population,
            red,
            green,
            blue,
        }
    }

    fn volume(&self) -> u32 {
        (self.red.1 - self.red.0 + 1) as u32
            * (self.green.1 - self.green.0 + 1) as u32
            * (self.blue.1 - self.blue.0 + 1) as u32
    }

    fn split_box(mut self) -> (Vbox<'a, P>, Vbox<'a, P>) {
        assert!(self.can_split());

        let split_point = self.find_split_point();
        let (our, new) = self.colors.split_at_mut(split_point);

        let new_self = Vbox::new(our);
        let new_box = Vbox::new(new);

        (new_self, new_box)
    }

    fn find_split_point(&mut self) -> usize {
        let longest_dimension = self.get_longest_dimension();

        self.colors.sort_by(|(lhs, _), (rhs, _)| match longest_dimension {
            Component::Red => lhs.to_rgb().0[0].cmp(&rhs.to_rgb().0[0]),
            Component::Green => lhs.to_rgb().0[1].cmp(&rhs.to_rgb().0[1]),
            Component::Blue => lhs.to_rgb().0[2].cmp(&rhs.to_rgb().0[2]),
        });

        let midpoint = self.population / 2;
        let mut pop = 0;

        for (i, (_, count)) in self.colors.iter().enumerate() {
            pop += count;

            if pop >= midpoint {
                return (self.colors.len() - 1).min(i);
            }
        }

        0
    }

    fn can_split(&self) -> bool {
        self.colors.len() > 1
    }

    fn get_longest_dimension(&self) -> Component {
        let red_length = self.red.1 - self.red.0;
        let green_length = self.green.1 - self.green.0;
        let blue_length = self.blue.1 - self.blue.0;

        match red_length.max(green_length).max(blue_length) {
            v if v == red_length => Component::Red,
            v if v == green_length => Component::Green,
            v if v == blue_length => Component::Blue,
            _ => panic!("impossible case"),
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
                        red_sum + quantize_color_channel(r) as u32 * count,
                        green_sum + quantize_color_channel(g) as u32 * count,
                        blue_sum + quantize_color_channel(b) as u32 * count,
                    )
                });

        let red_mean = (red_sum as f32 / pop as f32) as u8;
        let green_mean = (green_sum as f32 / pop as f32) as u8;
        let blue_mean = (blue_sum as f32 / pop as f32) as u8;

        let red_quantized = modify_word_with(red_mean, QUANTIZE_WORD_MASK, 8);
        let green_quantized = modify_word_with(green_mean, QUANTIZE_WORD_WIDTH, 8);
        let blue_quantized = modify_word_with(blue_mean, QUANTIZE_WORD_WIDTH, 8);

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
        self.volume().cmp(&other.volume())
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

fn quantize_color_channel(value: u8) -> u8 {
    (value >> QUANTIZE_WORD_WIDTH) & QUANTIZE_WORD_MASK
}

fn modify_word_with(value: u8, current_width: u8, target_width: u8) -> u8 {
    let new_value = if target_width > current_width {
        value << (target_width - current_width)
    } else {
        value >> (current_width - target_width)
    };

    new_value & ((1 << target_width) - 1)
}

fn split_boxes<P>(pq: &mut BinaryHeap<Vbox<'_, P>>, max_size: usize)
where
    P: image::Pixel<Subpixel = u8> + std::cmp::Eq + std::hash::Hash,
{
    while pq.len() < max_size {
        if let Some(vbox) = pq.pop() {
            if vbox.can_split() {
                let (old, split) = vbox.split_box();

                pq.push(split);
                pq.push(old);

                continue;
            }
        }

        return;
    }
}
