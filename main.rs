extern crate maps;

use maps::Map;

fn main() {
    let m = Map::new_biome(50, 50);
    //let m = Map::new_biome(50, 50, String::from("Ocean"));
    Map::save_map("resources/maps/test4.map", &m, false);
    //Map::save_map("resources/maps/test101.map", &m, false);
}
