use tundra_textures::utils;

pub fn main() {
    let texture_path1 = "/media/jakubvondra/Data/dev/muskox/tests/grid.tx";
    let texture_path2 =
        "/media/jakubvondra/Data/dev/tundra/repos/tundra-textures/examples/test.png";
    let texture_path2 =
        "/media/jakubvondra/Data/dev/tundra/repos/tundra-textures/examples/udimtest.<udim>.png";
    let mut cache = tundra_textures::TextureCache::empty();
    cache.add(texture_path1.to_string(), &utils::ColorSpace::Srgb, false);
    cache.add(texture_path2.to_string(), &utils::ColorSpace::Srgb, true);
}
