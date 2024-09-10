extern crate half;
extern crate rand;

use exr::block::reader::ChunksReader;
use exr::math::Vec2;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// exr imports
extern crate exr;

/// Collects the average pixel value for each channel.
/// Does not load the whole image into memory at once: only processes the image block by block.
/// On my machine, this program analyzes a 3GB file while only allocating 1.1MB.
pub fn load_tile_data(file_path: &str, mip_map_lvl: usize, tile_index: Vec2<usize>) -> Vec<f32> {
    use exr::prelude::*;

    let file = BufReader::new(File::open(file_path).expect("failed to read texture"));

    let start_time = ::std::time::Instant::now();

    // -- read the file, summing up the average pixel values --

    // start reading the file, extracting the meta data of the image
    let reader = exr::block::read(file, true).unwrap();

    // print progress only if it advances more than 1%
    let mut current_progress_percentage = 0;

    let mut rgba_values: Vec<f32> = Vec::new();

    // create a reader that loads only relevant chunks from the file, and also prints something on progress
    let reader = reader
        // filter out only the mip map level and tile we are interested in loading
        .filter_chunks(true, |meta_data, tile, block| {
            let header = &meta_data.headers[block.layer];
            //println!("lvl:{:?} tile:{:?}",block.level, tile.tile_index);
            tile.tile_index == tile_index && tile.level_index == Vec2(mip_map_lvl, mip_map_lvl)
        })
        .unwrap();

    // read all pixel blocks from the image, decompressing in parallel
    reader
        .decompress_parallel(true, |meta_data, block| {
            let header = &meta_data.headers[block.index.layer];
            //println!("{:?}", header.channels.list);
            // collect all pixel values from the pixel block
            let mut l_c = 0;
            //println!("{:?}",&header.channels);
            let n_channels = header.channels.list.len();
            //println!("number of channels: {:?}", n_channels);
            // read and store lines, where each read line holds data for one channel

            match n_channels {
                //TOOD handle better than turning into 4 channels
                1 => {
                    let mut y_lines: Vec<f32> = Vec::new();
                    for line in block.lines(&header.channels) {
                        let channel_i = line.location.channel;
                        let channel = &header.channels.list[channel_i];

                        let mut line_values: Vec<f32> = match channel.sample_type {
                            SampleType::F16 => line
                                .read_samples::<f16>()
                                .map(|v| v.unwrap().to_f32())
                                .collect(),

                            SampleType::F32 => {
                                line.read_samples::<f32>().map(|v| v.unwrap()).collect()
                            }

                            SampleType::U32 => line
                                .read_samples::<f32>()
                                .map(|v| v.unwrap() as f32)
                                .collect(),
                        };
                        y_lines.append(&mut line_values);
                    }
                    for i in 0..y_lines.len() {
                        rgba_values.push(y_lines[i]);
                        rgba_values.push(y_lines[i]);
                        rgba_values.push(y_lines[i]);
                        rgba_values.push(y_lines[i]); // TODO handle alpah better, add option for just rgb
                    }
                }
                //TOOD handle better than turning into 4 channels
                3 => {
                    let mut bgr_lines: [Vec<f32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
                    for line in block.lines(&header.channels) {
                        let channel_i = line.location.channel;
                        let channel = &header.channels.list[channel_i];

                        let mut line_values: Vec<f32> = match channel.sample_type {
                            SampleType::F16 => line
                                .read_samples::<f16>()
                                .map(|v| v.unwrap().to_f32())
                                .collect(),

                            SampleType::F32 => {
                                line.read_samples::<f32>().map(|v| v.unwrap()).collect()
                            }

                            SampleType::U32 => line
                                .read_samples::<f32>()
                                .map(|v| v.unwrap() as f32)
                                .collect(),
                        };
                        bgr_lines[channel_i].append(&mut line_values);
                    }
                    for i in 0..bgr_lines[0].len() {
                        rgba_values.push(bgr_lines[2][i]);
                        rgba_values.push(bgr_lines[1][i]);
                        rgba_values.push(bgr_lines[0][i]);
                        rgba_values.push(bgr_lines[0][i]); // TODO handle alpah better, add option for just rgb
                    }
                }
                4 => {
                    let mut abgr_lines: [Vec<f32>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for line in block.lines(&header.channels) {
                        let channel_i = line.location.channel;
                        let channel = &header.channels.list[channel_i];

                        let mut line_values: Vec<f32> = match channel.sample_type {
                            SampleType::F16 => line
                                .read_samples::<f16>()
                                .map(|v| v.unwrap().to_f32())
                                .collect(),

                            SampleType::F32 => {
                                line.read_samples::<f32>().map(|v| v.unwrap()).collect()
                            }

                            SampleType::U32 => line
                                .read_samples::<f32>()
                                .map(|v| v.unwrap() as f32)
                                .collect(),
                        };
                        abgr_lines[channel_i].append(&mut line_values);
                        //println!("channel: {:?}", channel);
                        // reshuffle into new vector in a R,G,B,A,R,G,B,A,R,G,B.... form
                    }
                    for i in 0..abgr_lines[0].len() {
                        rgba_values.push(abgr_lines[3][i]);
                        rgba_values.push(abgr_lines[2][i]);
                        rgba_values.push(abgr_lines[1][i]);
                        rgba_values.push(abgr_lines[0][i]);
                    }
                }

                _ => {}
            };

            //println!("lines: {:?}, n of values: {:?}", l_c, values.len());
            Ok(())
        })
        .unwrap();
    //println!("values: {:?}", values.len());

    rgba_values
}

pub fn parent_folder(filepath: String) -> String {
    let path = Path::new(&filepath);
    // Get the parent directory
    if let Some(parent) = path.parent() {
        parent.to_str().unwrap().to_string()
    } else {
        "".to_string()
    }
}

use regex::Regex;

pub fn tags_to_pattern(filepath: String) -> String {
    let replacement = "*";
    // Create a regex pattern that matches anything between '<' and '>'
    let re = Regex::new(r"<[^>]*>").unwrap();
    // Replace anything between '<' and '>' with the replacement text
    let result = re.replace_all(&filepath, format!("{}", replacement));
    result.to_string()
}

use glob::{glob, Pattern};

pub fn list_files_by_pattern(pattern: String) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // Iterate over the files that match the pattern
    for entry in glob(pattern.as_str().as_ref()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    out.push(path.to_str().unwrap().to_string());
                }
            }
            Err(e) => println!("Error: {}", e), // Handle errors, like invalid paths
        }
    }
    out
}
