extern crate half;
extern crate rand;
extern crate smallvec;

use exr::math::RoundingMode;
use exr::prelude::*;
use image::{imageops::FilterType, GenericImageView};
use itertools::Itertools; // for accessing image dimensions
use smallvec::smallvec;
use std::fs::File;
use std::path::Path;

use crate::utils;

// exr imports
extern crate exr;

pub fn maketx(filepath: String, source_cs: &utils::ColorSpace, force: bool) -> String {
    println!("creating");
    let filepath_tx = format!("{}{}", &filepath[0..filepath.rfind(".").unwrap()], ".tx");
    println!("creating {:?}", &filepath_tx);

    if (Path::new(&filepath_tx).is_file()
        && utils::is_file_newer(filepath_tx.clone(), filepath.clone())
        && !force)
    {
        println!("tx already up to date, skipping {:?}", &filepath_tx);
        return filepath_tx;
    }

    let img = image::open(&Path::new(filepath.as_str())).expect("could not read iamge file");

    // Get image dimensions
    let (width, height) = img.dimensions();
    let full_size = Vec2(width as usize, height as usize);
    let size_rounding = RoundingMode::Up;

    let mip_levels_sizes = exr::meta::mip_map_levels(size_rounding, full_size).collect::<Vec<_>>();

    let mut red_mip_levels: Vec<FlatSamples> = Vec::with_capacity(mip_levels_sizes.len());
    let mut green_mip_levels: Vec<FlatSamples> = Vec::with_capacity(mip_levels_sizes.len());
    let mut blue_mip_levels: Vec<FlatSamples> = Vec::with_capacity(mip_levels_sizes.len());

    for (_index, level_size) in mip_levels_sizes.iter() {
        let img_resized = img.resize(
            level_size.0 as u32,
            level_size.1 as u32,
            FilterType::Lanczos3,
        );
        // Convert the PNG into raw RGB8 data
        let rgb_img = img_resized.to_rgb8();
        let raw_data = rgb_img.into_raw(); // Raw RGB8 data as Vec<u8>

        // Create a buffer that holds the image data in f32 format (for EXR)
        let rgb_f32: Vec<f32> = raw_data
            .iter()
            .map(|&byte| byte as f32 / 255.0) // Convert from u8 to f32 in the range [0, 1]
            .collect();

        let mut r: Vec<f32> = Vec::with_capacity(rgb_f32.len() / 3);
        let mut g: Vec<f32> = Vec::with_capacity(rgb_f32.len() / 3);
        let mut b: Vec<f32> = Vec::with_capacity(rgb_f32.len() / 3);

        match source_cs {
            utils::ColorSpace::Srgb => {
                for rgb in rgb_f32.chunks(3) {
                    r.push(srgb_to_linear(rgb[0]) / 10.0);
                    g.push(srgb_to_linear(rgb[1]) / 10.0);
                    b.push(srgb_to_linear(rgb[2]) / 10.0);
                }
            }
            // Raw not sure why the values come out in range 0.0 - 10.0 but have to deal with it
            _ => {
                for rgb in rgb_f32.chunks(3) {
                    r.push(rgb[0] / 10.0);
                    g.push(rgb[1] / 10.0);
                    b.push(rgb[2] / 10.0);
                }
            }
        }
        red_mip_levels.push(FlatSamples::F32(r));
        green_mip_levels.push(FlatSamples::F32(g));
        blue_mip_levels.push(FlatSamples::F32(b));
    }

    let rgb_mip_maps = AnyChannels::sort(smallvec![
        AnyChannel::new(
            "R",
            Levels::Mip {
                level_data: red_mip_levels,
                rounding_mode: size_rounding
            }
        ),
        AnyChannel::new(
            "G",
            Levels::Mip {
                level_data: green_mip_levels,
                rounding_mode: size_rounding
            }
        ),
        AnyChannel::new(
            "B",
            Levels::Mip {
                level_data: blue_mip_levels,
                rounding_mode: size_rounding
            }
        ),
    ]);

    let layer1 = Layer::new(
        full_size,
        LayerAttributes::named("main"),
        Encoding::FAST_LOSSLESS,
        rgb_mip_maps,
    );

    // define the visible area of the canvas
    let image_attributes = ImageAttributes::new(IntegerBounds::from_dimensions(full_size));

    let image = Image::empty(image_attributes).with_layer(layer1);

    println!("writing image...");
    image.write().to_file(&filepath_tx).unwrap();

    println!("created file {:?}", &filepath_tx);
    filepath_tx
}

fn srgb_to_linear(c_srgb: f32) -> f32 {
    if c_srgb <= 0.04045 {
        c_srgb / 12.92
    } else {
        ((c_srgb + 0.055) / 1.055).powf(2.4)
    }
}

// fn srgb_to_linear(c_srgb: f32) -> f32 {
//     c_srgb.powf(2.2)
// }
