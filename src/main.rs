use clap::Parser;
use colored::Colorize;
use image::{GenericImageView, ImageReader, imageops::FilterType};
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    image: String,

    #[arg(short, long, default_value = "ascii")]
    mode: String,

    #[arg(short = 'n', long, default_value_t = false)]
    invert: bool,

    #[arg(short = 'w', long, default_value_t = 100)]
    width: u32,

    #[arg(short, long, default_value_t = 1.0)]
    contrast: f32,

    #[arg(short, long, default_value_t = false)]
    background: bool,
}

fn boost_saturation(val: u8, factor: f32) -> u8 {
    ((val as f32 * factor).min(255.0)) as u8
}

fn calculate_contrast(val: u8, factor: f32) -> u8 {
    if (factor - 1.0).abs() < f32::EPSILON {
        return val;
    }
    let normalized = val as f32 / 255.0;
    let adjusted = normalized.powf(factor);
    (adjusted * 255.0).clamp(0.0, 255.0) as u8
}

fn choose_ascii_format(mode: &str) -> (Vec<char>, bool) {
    const BLOCKS: &str = "█▓▒░ ";
    const BUBBLES: &str = "●◕○◌ ";
    const GEOMETRIC: &str = "■□▪▫ ";
    const ASCII_CHARS: &str =
        "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. ";

    match mode {
        "blocks" => (BLOCKS.chars().collect(), true),
        "bubbles" => (BUBBLES.chars().collect(), true),
        "geometric" => (GEOMETRIC.chars().collect(), true),
        "ascii" => (ASCII_CHARS.chars().collect(), false),
        _ => {
            eprintln!("Unknown format '{}', defaulting to ascii.", mode);
            (ASCII_CHARS.chars().collect(), false)
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let contrast_lut: Vec<u8> = (0..=255)
        .map(|i| calculate_contrast(i, args.contrast))
        .collect();

    let img = ImageReader::open(&args.image)?.decode()?;

    let target_width = args.width;
    let aspect_ratio = img.height() as f32 / img.width() as f32;
    let font_correction = 0.5;

    let target_height = (target_width as f32 * aspect_ratio * font_correction) as u32;
    let resized_img = img.resize_exact(target_width, target_height, FilterType::Lanczos3);

    let (palette, invert_palette) = choose_ascii_format(&args.mode);
    let palette_len = palette.len();

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout.lock());

    for y in 0..target_height {
        for x in 0..target_width {
            let pixel = resized_img.get_pixel(x, y);
            let [mut r, mut g, mut b, alpha] = pixel.0;

            if args.invert {
                r = 255 - r;
                g = 255 - g;
                b = 255 - b;
            }

            if alpha < 50 || (r < 30 && g < 30 && b < 30) {
                write!(handle, " ")?;
                continue;
            }

            let brightness = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) as u8;

            let contrasted = contrast_lut[brightness as usize];

            let mut char_index = (contrasted as usize * (palette_len - 1)) / 255;

            if invert_palette {
                char_index = palette_len - 1 - char_index;
            }

            char_index = char_index.min(palette_len - 1);
            let symbol = palette[char_index];

            let r_boost = boost_saturation(r, 1.5);
            let g_boost = boost_saturation(g, 1.5);
            let b_boost = boost_saturation(b, 1.5);

            if args.background {
                write!(
                    handle,
                    "{}",
                    symbol
                        .to_string()
                        .truecolor(r_boost, g_boost, b_boost)
                        .on_truecolor(r / 2, g / 2, b / 2)
                )?;
            } else {
                write!(
                    handle,
                    "{}",
                    symbol.to_string().truecolor(r_boost, g_boost, b_boost)
                )?;
            }
        }
        writeln!(handle)?;
    }
    handle.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boost_saturation_no_overflow() {
        assert_eq!(boost_saturation(100, 1.0), 100);
        assert_eq!(boost_saturation(100, 2.0), 200);
        assert_eq!(boost_saturation(200, 2.0), 255);
    }

    #[test]
    fn test_calculate_contrast_identity() {
        assert_eq!(calculate_contrast(128, 1.0), 128);
    }

    #[test]
    fn test_calculate_contrast_increase() {
        let result = calculate_contrast(128, 2.0);
        assert!(result < 128);
    }

    #[test]
    fn test_choose_ascii_format_ascii() {
        let (palette, invert) = choose_ascii_format("ascii");
        assert!(!palette.is_empty());
        assert!(!invert);
    }

    #[test]
    fn test_choose_ascii_format_blocks() {
        let (palette, invert) = choose_ascii_format("blocks");
        assert!(!palette.is_empty());
        assert!(invert);
    }
}
