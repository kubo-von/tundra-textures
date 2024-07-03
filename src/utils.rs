extern crate half;
extern crate rand;

use exr::block::reader::ChunksReader;
use exr::math::Vec2;
use std::fs::File;
use std::io::BufReader;

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

            // read and store lines, where each read line holds data for one channel
            let mut abgr_lines: [Vec<f32>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
            for line in block.lines(&header.channels) {
                let channel = line.location.channel;
                let mut line_values: Vec<f32> =
                    line.read_samples::<f32>().map(|v| v.unwrap()).collect();
                abgr_lines[channel].append(&mut line_values);
                //println!("channel: {:?}", channel);
            }

            // reshuffle into new vector in a R,G,B,A,R,G,B,A,R,G,B.... form
            for i in 0..abgr_lines[0].len() {
                rgba_values.push(abgr_lines[3][i]);
                rgba_values.push(abgr_lines[2][i]);
                rgba_values.push(abgr_lines[1][i]);
                rgba_values.push(abgr_lines[0][i]);
            }

            //println!("lines: {:?}, n of values: {:?}", l_c, values.len());
            Ok(())
        })
        .unwrap();
    //println!("values: {:?}", values.len());

    rgba_values
}
