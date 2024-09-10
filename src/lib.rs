// To generate tiled texture
// export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:/media/jakubvondra/Data/apps/gafferHQ/gaffer-1.4.6.0-linux-gcc11/lib/"
// ./maketx --format exr /media/jakubvondra/Data/dev/muskox/tests/grid.exr
// cargo run --release --example load

pub use exr;
use exr::math::Vec2;
use glam;
use hashbrown::HashMap;
use std::sync::{Arc, RwLock};

mod txmake;
mod utils;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub struct TextureCache {
    pub textures: HashMap<String, Arc<RwLock<Texture>>>,
}
impl TextureCache {
    pub fn empty() -> Self {
        TextureCache {
            textures: HashMap::new(),
        }
    }
    pub fn add(&mut self, texture_path: String) {
        let mut texture_paths: Vec<String> = Vec::new();
        if texture_path.contains("<") {
            let pattern = utils::tags_to_pattern(texture_path);
            let mut matching = utils::list_files_by_pattern(pattern);
            texture_paths.append(&mut matching);
        } else {
            texture_paths.push(texture_path)
        }

        for p in texture_paths.iter() {
            let tx_path = match p.ends_with(".tx") {
                true => p.clone(),
                false => txmake::maketx(p.clone()),
            };

            &self.textures.insert(
                tx_path.clone(),
                Arc::new(RwLock::new(Texture::new(tx_path))),
            );
        }
    }
}

pub struct Texture {
    pub path: String,
    pub resolution: (usize, usize),
    pub mipmaps: Vec<MipMap>,
}
impl Texture {
    pub fn new(file_path: String) -> Self {
        let metadata = exr::meta::MetaData::read_from_file(file_path.clone(), false)
            .expect(format!("could not read metadata from {:?}", file_path).as_str());
        //println!("{:?}", metadata.headers[0].layer_size);
        //println!("{:?}",metadata.headers[0].channels);

        let resolution = metadata.headers[0].layer_size;

        let mipmaps = exr::meta::mip_map_levels(exr::math::RoundingMode::Down, resolution)
            .map(|m| MipMap::empty(m.1))
            .collect();
        println!("added to texture cache: {:?}", &file_path);
        Texture {
            path: file_path,
            resolution: (resolution.0, resolution.1),
            mipmaps: mipmaps,
        }
    }

    // checks if tile for given sample is already loaded or needs loading, returns tile index and status - true loaded, false - needs loading
    pub fn tile_loaded(&self, uv: Vec2<f32>, mipmap_lvl: usize) -> (bool, Vec2<usize>, usize) {
        // (is loaded ?, tile position, tile_index)
        let mip_map_resolution = *&self.mipmaps[mipmap_lvl].resolution;

        //println!("mm: {:?}, t_pos: {:?}, t_i: {:?}, n_ti: {:?}",mipmap_lvl,tile_pos, tile_index, n_tiles_x);

        // pixel postion relative to the mip map
        let mipmap_pixel_pos = Vec2(
            (uv.x() * (mip_map_resolution.y()) as f32 - 1.0).round() as usize,
            (uv.y() * (mip_map_resolution.x()) as f32 - 1.0).round() as usize,
        );

        let tile_pos = Vec2(mipmap_pixel_pos.x() / 64, mipmap_pixel_pos.y() / 64);
        let tile_index =
            tile_pos.x() + tile_pos.y() * self.mipmaps[mipmap_lvl].tiles_n.x() as usize;

        (
            self.mipmaps[mipmap_lvl].tiles[tile_index].is_some(),
            tile_pos,
            tile_index,
        )
    }

    pub fn tile_load(&mut self, mipmap_lvl: usize, tile_index: usize, tile_pos: Vec2<usize>) {
        let mip_map_resolution = *&self.mipmaps[mipmap_lvl].resolution;

        if self.mipmaps[mipmap_lvl].tiles[tile_index].is_none() {
            let rgba_values = utils::load_tile_data(self.path.as_str(), mipmap_lvl, tile_pos);
            let tile_size = Vec2(
                exr::meta::calculate_block_size(mip_map_resolution.x(), 64, tile_pos.x()).unwrap(),
                exr::meta::calculate_block_size(mip_map_resolution.y(), 64, tile_pos.y()).unwrap(),
            );
            self.mipmaps[mipmap_lvl].tiles[tile_index] = Some(Tile {
                tile_size: tile_size, //TODO set actual tile size
                pixels: rgba_values,
            });
        }
    }

    pub fn sample(
        &self,
        uv: Vec2<f32>,
        mipmap_lvl: usize,
        tile_pos: Vec2<usize>,
        tile_index: usize,
    ) -> glam::Vec4 {
        if mipmap_lvl >= self.mipmaps.len() {
            return glam::Vec4::ZERO;
        }
        let mip_map_resolution = *&self.mipmaps[mipmap_lvl].resolution;

        // pixel postion relative to the mip map
        let mipmap_pixel_pos = Vec2(
            (uv.x() * (mip_map_resolution.y()) as f32 - 1.0).round() as usize,
            (uv.y() * (mip_map_resolution.x()) as f32 - 1.0).round() as usize,
        );

        // pixel postion relative to the tile
        let tile_pixel_pos = Vec2(
            mipmap_pixel_pos.x() - tile_pos.x() * 64,
            mipmap_pixel_pos.y() - tile_pos.y() * 64,
        );

        let tile_pixel_index = tile_pixel_pos.x()
            + tile_pixel_pos.y()
                * &self.mipmaps[mipmap_lvl].tiles[tile_index]
                    .as_ref()
                    .unwrap()
                    .tile_size
                    .x();
        //println!("uv: {:?}, tile index: {:?}, tile size: {:?}, tile_pos: {:?}, mipmap_pixel_pos: {:?}, tile_pixel_pos: {:?}, tile_pixel_index: {:?}",uv, tile_index,&self.mipmaps[mipmap_lvl].tiles[tile_index].as_ref().unwrap().tile_size,tile_pos,mipmap_pixel_pos,tile_pixel_pos,tile_pixel_index);

        //glam::Vec3::new(uv.x()*1.0, uv.y()*1.0, tile_id as f32 / ((n_tiles_x*n_tiles_y) as f32) )
        let pixel_values = &self.mipmaps[mipmap_lvl].tiles[tile_index]
            .as_ref()
            .expect("trying to read tile that is not loaded yet")
            .pixels;

        glam::Vec4::new(
            pixel_values[tile_pixel_index * 4],
            pixel_values[tile_pixel_index * 4 + 1],
            pixel_values[tile_pixel_index * 4 + 2],
            pixel_values[tile_pixel_index * 4 + 3],
        )
    }
}

pub struct MipMap {
    pub resolution: Vec2<usize>,
    pub tiles_size: Vec2<usize>,
    pub tiles_n: Vec2<usize>,
    pub tiles: Vec<Option<Tile>>,
}
impl MipMap {
    pub fn empty(resolution: Vec2<usize>) -> Self {
        let tile_size = Vec2(64, 64);

        let tiles_n = Vec2(
            exr::meta::compute_block_count(resolution.x(), tile_size.x()),
            exr::meta::compute_block_count(resolution.y(), tile_size.y()),
        );

        let n_tiles = tiles_n.x() * tiles_n.y();

        let tiles = (0..n_tiles).map(|tile_i| None).collect();

        //println!("mimap {:?} - {:?} tiles", resolution, n_tiles);

        MipMap {
            resolution: resolution,
            tiles_size: tile_size,
            tiles_n: tiles_n,
            tiles: tiles,
        }
    }
}

pub struct Tile {
    tile_size: Vec2<usize>,
    pixels: Vec<f32>,
}
impl Tile {}
