use exr::math::Vec2;
use glam::{Vec3, Vec4};
use minifb::{Key, ScaleMode, Window, WindowOptions};
use peak_alloc::PeakAlloc;
use rand::prelude::*;
use std::sync::mpsc::channel;
use std::{f32, thread, time};
use threadpool::ThreadPool;
use tundra_textures;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

pub fn main() {
    let texture_path = "/media/jakubvondra/Data/dev/tundra/repos/tundra-textures/examples/test.tx";
    //let texture_path = "/media/jakubvondra/Data/dev/test_data/exrs/mipmap/debug.tx";

    let mut cache = tundra_textures::TextureCache::empty();
    cache.add(texture_path.to_string(), &utils::ColorSpace::Srgb);

    let test_texture_arc = cache.textures.get(texture_path).unwrap().clone();
    let test_texture_arc_clone = test_texture_arc.clone();
    // spawn new threads
    let pool = ThreadPool::new(2);

    // start a first thread which will try to randmoly access texture tiles and load them if they are nto loaded yeat
    pool.execute(move || {
        //thread::sleep(time::Duration::from_secs(1));
        let mut rng = rand::thread_rng();
        thread::sleep(time::Duration::from_secs(5));
        for i in 0..5000 {
            // add some sleep so they are nto loaded all at once
            thread::sleep(time::Duration::from_millis(100));
            let mut test_texture_W = test_texture_arc_clone.write().unwrap();
            let mut loaded = true;
            let mut tile_index = 0;
            let mut tile_pos = Vec2(0, 0);
            let mut lvl = 0;

            // try to load some random tile which is not loaded yet
            let mut li = 0;
            while loaded {
                let uv: Vec2<f32> = Vec2(rng.gen(), rng.gen());
                lvl = rng.gen_range(0..8);
                (loaded, tile_pos, tile_index) = test_texture_W.tile_loaded(uv, lvl as usize);
                li += 1;
                // eascape after 1000 tries, image probably fully loaded
                if li == 1000 {
                    break;
                }
            }
            if !loaded {
                test_texture_W.tile_load(lvl, tile_index, tile_pos);
            }
            drop(test_texture_W);
        }
    });

    // STarting a second thread with mini fb window which is showing the tiles if they are loaded
    let test_texture_arc_clone2 = test_texture_arc.clone();
    pool.execute(move || {
        //thread::sleep(time::Duration::from_secs(1));
        let test_texture_R = test_texture_arc_clone2.read().unwrap();

        let window_size = (test_texture_R.resolution.0 * 2, test_texture_R.resolution.1);

        let mut buffer = vec![0u32; window_size.0 * window_size.1];

        let mut mm_bboxes: Vec<(Vec2<usize>, Vec2<usize>)> = Vec::new();
        let x_offset = test_texture_R.resolution.0;
        let mut y_offset = 0;
        for (mi, mm) in test_texture_R.mipmaps.iter().enumerate() {
            if mi == 0 {
                mm_bboxes.push((
                    Vec2(0, 0),
                    Vec2(test_texture_R.resolution.0, test_texture_R.resolution.1),
                ))
            } else {
                mm_bboxes.push((
                    Vec2(x_offset, y_offset),
                    Vec2(x_offset + mm.resolution.0, y_offset + mm.resolution.1),
                ));
                y_offset += mm.resolution.1;
            }
        }
        //println!("{:?}", mm_bboxes);
        drop(test_texture_R);

        let mut window = Window::new(
            "Noise Test - Press ESC to exit",
            window_size.0,
            window_size.1,
            WindowOptions {
                resize: false,
                scale_mode: ScaleMode::UpperLeft,
                ..WindowOptions::default()
            },
        )
        .expect("Unable to create the window");

        window.set_target_fps(60);
        let current_mem = ((PEAK_ALLOC.current_usage_as_mb() - 4.0) * 100.0).trunc() / 100.0;
        let title = format!("{} -- memory used: {} mb", texture_path, current_mem);
        window.set_title(title.as_str());

        let mut read_timer = 0;
        while window.is_open() && !window.is_key_down(Key::Escape) {
            if read_timer == 60 {
                thread::sleep(time::Duration::from_millis(3));

                match test_texture_arc.try_read() {
                    Ok(test_texture_R2) => {
                        //println!("update");
                        for (pi, pixel) in buffer.iter_mut().enumerate() {
                            let pixel_pos = Vec2(pi % window_size.0, pi / window_size.0);

                            let (mip_map_lvl, mm_pixel_pos) =
                                intersect_mipmap(&mm_bboxes, pixel_pos);
                            let mm_size =
                                test_texture_R2.mipmaps[mip_map_lvl.max(0) as usize].resolution;

                            let uv = Vec2(
                                mm_pixel_pos.x() as f32 / mm_size.x() as f32,
                                mm_pixel_pos.y() as f32 / mm_size.y() as f32,
                            );

                            let mut rgb = Vec4::ZERO;

                            if mip_map_lvl >= 0 {
                                let l = ((mip_map_lvl + 1) as f32 / 0.368844516).fract() * 0.3;
                                rgb = Vec4::new(l, l, l, 1.0);

                                let (loaded, tile_pos, tile_index) =
                                    test_texture_R2.tile_loaded(uv, mip_map_lvl as usize);
                                if loaded {
                                    rgb = test_texture_R2.sample(
                                        uv,
                                        mip_map_lvl as usize,
                                        tile_pos,
                                        tile_index,
                                    );
                                } else {
                                    // texture.tile_load(mip_map_lvl as usize, tile_index, tile_pos);
                                    // rgb = texture.sample(uv,mip_map_lvl as usize, tile_pos, tile_index);
                                }
                            }

                            *pixel = float_rgb_to_32bit(rgb.x, rgb.y, rgb.z, 1.0);
                        }
                    }
                    Err(_) => {
                        read_timer = 0;
                    }
                }
                read_timer = 0;
                let current_mem =
                    ((PEAK_ALLOC.current_usage_as_mb() - 4.0) * 100.0).trunc() / 100.0;
                let title = format!("{} -- memory used: {} mb", texture_path, current_mem);
                window.set_title(title.as_str());
            }
            read_timer += 1;
            window
                .update_with_buffer(&buffer, window_size.0, window_size.1)
                .unwrap();
        }
    });

    pool.join();
}

fn float_rgb_to_32bit(r: f32, g: f32, b: f32, a: f32) -> u32 {
    // Ensure the values are within the range [0.0, 1.0]
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    let a = a.clamp(0.0, 1.0);

    // Convert to 8-bit values and pack into a 32-bit integer
    let r = (r * 255.0).round() as u32;
    let g = (g * 255.0).round() as u32;
    let b = (b * 255.0).round() as u32;
    let a = (a * 255.0).round() as u32;

    (a << 24) | (r << 16) | (g << 8) | b
}

// find if pixel is in some mipmap's bbox
fn intersect_mipmap(
    bboxes: &Vec<(Vec2<usize>, Vec2<usize>)>,
    pixel_pos: Vec2<usize>,
) -> (i32, Vec2<usize>) {
    for (mi, bbox) in bboxes.iter().enumerate() {
        if pixel_pos.x() >= bbox.0.x()
            && pixel_pos.x() <= bbox.1.x()
            && pixel_pos.y() >= bbox.0.y()
            && pixel_pos.y() <= bbox.1.y()
        {
            return (
                mi as i32,
                Vec2(pixel_pos.x() - bbox.0.x(), pixel_pos.y() - bbox.0.y()),
            );
        }
    }
    (-1, Vec2(0, 0))
}
