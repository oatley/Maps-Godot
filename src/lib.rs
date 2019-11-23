#[macro_use]
extern crate gdnative;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate flate2;

use gdnative::*;
use rand::Rng;
use std::collections::HashMap;
use std::string::String;
use serde::{Serialize, Deserialize};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

// To do:
// - Store biome control and tile chance inside Map (probably not)
// - combine biome control and tile chance into single function or creation (maybe)
// - I don't like how tilechance.floor, tile_type.floor are all unconnected... (too bad)
// -- All three of the above to come together in a single hashmap or data structure of some kind (nope)
// - Map can be restructured for simplicity and security: priv Map, pub GodotMap, priv Save/Load/CompressMap (yes please refactor)
// -- Separating map into the generation of the tileset, godot interface, and extra tools for load/save/compress (next refactor)
// - Pathfinding:
// -- A tile thing with f = g + h, parent, and neighbors
// -- A hashmap(if we need keys) or vector(probably) with a list pathing tiles
// -- Can gdnative rust pathfind for godot without reading the map file from disk (???)

// Tree ideas were stupid, they are just tiles

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tile { // Individual tile data, stored in Map struct HashMap
    pub x: i32,
    pub y: i32,
    pub c: char,
    pub neighbors: Vec<String> // this will store a key to game_objects, for each neighbor tiles
}
pub struct TileChance { // Used to control the biome tiles on map
    pub floor: f32, // percentage of map floor
    pub wall: f32,  // percentage of map wall
    pub water: f32, // percentage of map water
    pub sand: f32,
    pub tree: f32
}
pub struct BiomeControl {
    pub water_edges: bool, // Activate method add_water_edges(), makes floor around water
    pub outer_wall: bool, // Activate method add_border_walls(), add wall around map
    pub sparse_trees: bool
}
pub struct Biome { // Used to control advanced biome manipulation
    pub biome_name: String,
    pub tile_chance: TileChance,
    pub biome_control: BiomeControl
}
pub struct TileType { // Static struct to store char for each type_type '.' '#' '~'
    pub floor: char,
    pub wall: char,
    pub water: char,
    pub sand: char, // Sand is just a variation on floor
    pub tree: char
}
static TILE_TYPE: TileType = TileType { // Static struct of TileType, avoid hardcode chars in methods
    floor: '.',
    wall: '#',
    water: '~',
    sand: ',',
    tree: 't'
};

#[derive(gdnative::NativeClass)]
#[inherit(gdnative::Node)]
pub struct Map {
    pub tileset: HashMap<String, Tile>
}

#[gdnative::methods]
impl Map {
    pub fn test() {
        // Convert godot string to rust string
        let m = Map::new_biome(50, 50, String::from("Forest"));
        Map::save_map("/tmp/maps/test101.map", &m, false);
    }
    fn new(tileset: HashMap<String, Tile>) -> Map {
        // Build Map structure
        let map: Map = Map {
            tileset: tileset
        };
        map
    }
    // Make required directories, give placeholder object to godot
    fn _init(_owner: Node) -> Self {
        let _dir = match fs::create_dir_all("/tmp/maps") { // Make new directory, don't error if exists
            Ok(_dir) => _dir,
            Err(_e) => (),
        };
        let tileset = HashMap::new();
        // Build Map structure
        let map = Map::new(tileset);
        map
    }
    // Not used
    #[export]
    fn _ready(&self, _owner: Node) {

    }
    #[export]
    unsafe fn godot_new_biome(&self, _owner: Node, godot_file_name: GodotString, godot_biome_name: GodotString) {
        // Convert godot string to rust string
        let file_name = godot_file_name.to_string();
        let biome_name = godot_biome_name.to_string();
        let m = Map::new_biome(50, 50, biome_name);
        Map::save_map(&file_name.to_string(), &m, false);
    }
    #[export]
    unsafe fn godot_random_biome(&self, _owner: Node, godot_file_name: GodotString) -> GodotString {
        let mut rng = rand::thread_rng();
        let random_biome = rng.gen_range(0, 5);
        let biome_name;
        if random_biome == 0 {
            biome_name = String::from("Cave");
        } else if random_biome == 1 {
            biome_name = String::from("Ocean");
        } else if random_biome == 2 {
            biome_name = String::from("Underlake");
        } else if random_biome == 3 {
            biome_name = String::from("Desert");
        } else if random_biome == 4 {
            biome_name = String::from("Forest");
        } else {
            biome_name = String::from("Cave");
        }
        let file_name = godot_file_name.to_string();
        let m = Map::new_biome(150, 50, biome_name.clone());
        Map::save_map(&file_name.to_string(), &m, false);
        // Return the random biome to godot for logging
        GodotString::from_str(&biome_name)
    }

    // Generate new map of a specific biome
    pub fn new_biome(sizex: i32, sizey: i32, biome_name: String) -> Map {
        // Setup basic map creation data
        let mut tileset: HashMap<String, Tile>; // Will store the final map data, exported to json
        let biome = Biome::new(biome_name); // Stores data that controls map creation
        // Prepare random data for voronoi point selection
        let mut rng = rand::thread_rng();
        let number_of_regions = rng.gen_range(sizex+sizey, (sizex+sizey)*2);
        let voronoi_regions: Vec<Tile>;
        // Pass 1: generate voronoi_regions using the TileChance to control biome creation
        voronoi_regions = Map::create_voronoi_points(sizex, sizey,  &biome, number_of_regions);
        // Pass 2: generate empty tileset
        tileset = Map::empty_tileset(sizex, sizey);
        // Pass 3: convert empty tileset to closest voronoi regions
        tileset = Map::tiles_to_voronoi(tileset, voronoi_regions);
        // Pass 4: update neighbors of each tile (include corners)
        tileset = Map::update_all_neighbors(sizex, sizey, tileset);
        // Pass 5: make sure all tiles around water are floor
        if biome.biome_control.water_edges {
            tileset = Map::add_water_edges(sizex, sizey, &biome, tileset);
        }
        // Pass 6: Tree sparseness (slow!) turn off for tilesets with little or no trees
        if biome.biome_control.sparse_trees {
            tileset = Map::add_sparse_trees(sizex, sizey, &biome, tileset);
        }
        // Pass FINAL: update wall_borders
        if biome.biome_control.outer_wall {
            tileset = Map::add_wall_borders(sizex, sizey, &biome, tileset);
        }
        // Pass X: triangulation (skipping)
        // Pass X: pathfinding

        // Sneaky secret tiles for map_size and default tiles
        let map_size = Tile::new(sizex, sizey, '$', Vec::new());
        let default_floor = Tile::new(sizex, sizey, biome.default_floor(), Vec::new());
        let default_wall = Tile::new(sizex, sizey, biome.default_wall(), Vec::new());
        tileset.insert(String::from("mapsize"), map_size);
        tileset.insert(String::from("default_floor"), default_floor);
        tileset.insert(String::from("default_wall"), default_wall);

        // Build Map structure
        let map = Map::new(tileset);
        map
    }

    // Create empty tileset of a specific size
    fn empty_tileset (sizex: i32, sizey: i32) -> HashMap<String,Tile> {
        let mut tileset = HashMap::new();
        // Generate temporary tiles
        for y in 0..sizey {
            for x in 0..sizex {
                let t = Tile::new(x, y, TILE_TYPE.floor, Vec::new());
                tileset.insert(t.get_tile_key(), t);
            }
        }
        tileset
    }

    // This is going to get awful and bloated fast! (maybe rewrite without structs) (think about it)
    // This could become a part of biome? I mean it is used specifically to change based on biome...
    // Is there good reason?
    fn create_voronoi_points(sizex: i32, sizey: i32, biome: &Biome, number_of_regions: i32) -> Vec<Tile> {
        // Get exact number of tiles needed for each type (from TileChance percentage)
        let number_of_floor = (biome.tile_chance.floor * number_of_regions as f32) as i32;
        let number_of_wall = (biome.tile_chance.wall * number_of_regions as f32) as i32;
        let number_of_water = (biome.tile_chance.water * number_of_regions as f32) as i32;
        let number_of_sand = (biome.tile_chance.sand * number_of_regions as f32) as i32;
        let number_of_trees = (biome.tile_chance.tree * number_of_regions as f32) as i32;
        let mut voronoi_regions = Vec::new();
        // new_voronoi_tiles returns the exact number of tiles requested of the specific type, xy positions are random
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_floor, TILE_TYPE.floor, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_wall, TILE_TYPE.wall, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_water, TILE_TYPE.water, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_sand, TILE_TYPE.sand, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_trees, TILE_TYPE.tree, voronoi_regions);
        voronoi_regions.push(Tile::new(sizex/2, sizey/2, biome.default_floor(), Vec::new())); // Player spawn
        voronoi_regions
    }

    // Convert empty tiles in tileset to closest voronoi region type
    fn tiles_to_voronoi (tileset: HashMap<String, Tile>, voronoi_regions: Vec<Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let mut closest_region: usize = 0;
            // Find closest voronoi region to current tile
            for region in 0..voronoi_regions.len() {
                let current_region: usize = region as usize;
                let diff = Tile::distance(&voronoi_regions[current_region], &tileset[tile_key]);
                let old_diff = Tile::distance(&voronoi_regions[closest_region], &tileset[tile_key]);
                if diff < old_diff {
                    closest_region = current_region;
                }
            }
            // Convert tile_type to voronoi region tile_type, store in new_tileset
            let new_tile = Tile::new(tileset[tile_key].x.clone(), tileset[tile_key].y.clone(), voronoi_regions[closest_region].c.clone() , Vec::new());
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Store side neighbors and corner neighbors
    fn update_all_neighbors (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            let tile_type = tileset[tile_key].c;
            let new_tile;
            if x <= 1 || y <= 1 || x >= sizex-1 || y >= sizey-1 {
                new_tile = Tile::new(x, y, tile_type, Vec::new());
            } else {
                let neighbors: Vec<String> = vec![
                    // Sides
                    (x + 1).to_string() + "x" + &(y).to_string(),
                    (x - 1).to_string() + "x" + &(y).to_string(),
                    (x).to_string() + "x" + &(y + 1).to_string(),
                    (x).to_string() + "x" + &(y - 1).to_string(),
                    // Corners
                    (x + 1).to_string() + "x" + &(y + 1).to_string(),
                    (x - 1).to_string() + "x" + &(y - 1).to_string(),
                    (x + 1).to_string() + "x" + &(y - 1).to_string(),
                    (x - 1).to_string() + "x" + &(y + 1).to_string()
                ];
                new_tile = Tile::new(x, y, tile_type, neighbors);
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Change all water tiles touching walls into floor (more walkable space)
    fn add_water_edges (sizex: i32, sizey: i32, biome: &Biome, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            let mut tile_type = tileset[tile_key].c;
            let neighbors = tileset[tile_key].neighbors.clone();
            let new_tile;
            if x <= 2 || y <= 2 || x >= sizex-2 || y >= sizey-2 {
                new_tile = Tile::new(x, y, tile_type, neighbors);
            } else {
                for neighbor_key in neighbors.clone() {
                    if tileset[tile_key].c == TILE_TYPE.water && (tileset[&neighbor_key].c == TILE_TYPE.wall || tileset[&neighbor_key].c == TILE_TYPE.tree) {
                        tile_type = biome.default_floor();
                    }
                }
                new_tile = Tile::new(x, y, tile_type, neighbors);
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Delete trees at random if they are touching too many trees
    fn add_sparse_trees (sizex: i32, sizey: i32, biome: &Biome, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        let mut rng = rand::thread_rng();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            let mut tile_type = tileset[tile_key].c;
            let neighbors = tileset[tile_key].neighbors.clone();
            let new_tile;
            if x <= 2 || y <= 2 || x >= sizex-2 || y >= sizey-2 {
                new_tile = Tile::new(x, y, tile_type, neighbors);
            } else {
                let mut number_of_trees = 1;
                for neighbor_key in neighbors.clone() {
                    if tileset[tile_key].c == TILE_TYPE.tree && (tileset[&neighbor_key].c == TILE_TYPE.wall || tileset[&neighbor_key].c == TILE_TYPE.tree) {
                        number_of_trees += 1
                    }
                }
                let delete_chance = rng.gen_range(0, number_of_trees);
                if delete_chance >= 3 {
                    tile_type = biome.default_floor();
                }
                new_tile = Tile::new(x, y, tile_type, neighbors);
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Convert all tiles found at edges of map to wall
    fn add_wall_borders (sizex: i32, sizey: i32, biome: &Biome, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            let neighbors = tileset[tile_key].neighbors.clone();
            let new_tile;
            if x == 0 || y == 0 || x == sizex-1 || y == sizey-1 {
                new_tile = Tile::new(x, y, biome.default_wall(), neighbors);
            } else {
                new_tile = tileset[tile_key].clone();
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Opens a file for reading to decompress, deserialize, and store as hashmap
    pub fn load_map(filename: &str, compression: bool) -> Map {
        let mut f = File::open(filename).expect("Unable to open file");
        let mut s = String::new();
        if compression {
            s = Map::decompress(&f);
        } else {
            f.read_to_string(&mut s).unwrap();
        }
        //GzDecoder::new(f).read_to_string(&mut s).unwrap();
        let tileset: HashMap<String, Tile> = serde_json::from_str(&s).unwrap();
        let map = Map::new(tileset);
        map
    }

    // Serialize hashmap into string, open a file for writing, write to file with compressed bufwriter
    pub fn save_map (filename: &str, map: &Map, compression: bool) {
        let serialized = serde_json::to_string(&map.tileset).unwrap();
        let f = File::create(filename).expect("Unable to create file");
        let enc: flate2::write::GzEncoder<std::fs::File>;
        // if compression enabled, gzip here
        if compression {
            enc = Map::compress(f);
            let mut buf = BufWriter::new(enc);
            buf.write_all(serialized.as_bytes()).expect("Unable to write data");
        } else {
            //enc = f;
            let mut buf = BufWriter::new(f);
            buf.write_all(serialized.as_bytes()).expect("Unable to write data");
        }
    }
    // Write wrapper to compress file, return encoder file
    pub fn compress(file: File) -> flate2::write::GzEncoder<std::fs::File>  {
        let enc = GzEncoder::new(file, Compression::default());
        enc
    }
    // Write wrapper to decompress file, return string
    pub fn decompress(f: &std::fs::File) -> String{
        let mut s = String::new();
        GzDecoder::new(f).read_to_string(&mut s).unwrap();
        s
    }

}

impl Tile {
    pub fn new(x: i32, y: i32, c: char, neighbors: Vec<String>) -> Tile {
        Tile { x: x, y: y, c: c, neighbors: neighbors }
    }
    // Return new vector filled with tiles, random xy positions, set specific tile type
    pub fn new_voronoi_tiles(sizex: i32, sizey: i32, number_of_tiles: i32, tile_type: char, mut voronoi_regions: Vec<Tile>) -> Vec<Tile> {
        let mut rng = rand::thread_rng();
        let mut tiles_remaining = number_of_tiles;
        while tiles_remaining > 0 {
            tiles_remaining -= 1;
            voronoi_regions.push(
                Tile::new(
                    rng.gen_range(0, sizex),
                    rng.gen_range(0, sizey),
                    tile_type,
                    Vec::new()
                )
            );
        }
        voronoi_regions
    }
    // Create new tile key string, xy coordinate with separator
    pub fn get_tile_key(&self) -> String {
        let tx = &self.x.to_string();
        let ty = &self.y.to_string();
        let sep = String::from("x");
        let mut s = String::new();
        s.push_str(&tx);
        s.push_str(&sep);
        s.push_str(&ty);
        s
    }
    // Calculate distance between tiles for v-regions in gen_map
    pub fn distance(v: &Tile, t: &Tile) -> i32 {
        let distance = (v.x - t.x).abs() + (v.y - t.y).abs();
        distance
    }
    // Calculate distance between tiles for v-regions in gen_map
    pub fn distance_slow(v: &Tile, t: &Tile) -> i32 {
        let x = (v.x - t.x).pow(2) as f64;
        let y = (v.y - t.y).pow(2) as f64;
        let distance = (x + y).sqrt() as i32;
        distance
    }
}

// Abstract biome a little so it's not just dumb data
impl Biome {
    fn new(biome_name: String) -> Biome {
        let biome;
        let tile_chance;
        let biome_control;
        if biome_name == "Cave" {
            tile_chance = TileChance{floor: 0.3, wall: 0.5, water: 0.2, sand: 0.0, tree: 0.0};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false};
        } else if biome_name == "Ocean" {
            tile_chance = TileChance{floor: 0.0, wall: 0.05, water: 0.7, sand: 0.15, tree: 0.1};
            biome_control = BiomeControl{outer_wall: false, water_edges: true, sparse_trees: true};
        } else if biome_name == "Underlake" {
            tile_chance = TileChance{floor: 0.2, wall: 0.2, water: 0.6, sand: 0.0, tree: 0.0};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false};
        } else if biome_name == "Desert" {
            tile_chance = TileChance{floor: 0.0, wall: 0.2, water: 0.15, sand: 0.5, tree: 0.15};
            biome_control = BiomeControl{outer_wall: false, water_edges: true, sparse_trees: true};
        } else if biome_name == "Forest" {
            tile_chance = TileChance{floor: 0.0, wall: 0.2, water: 0.2, sand: 0.2, tree: 0.4};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: true};
        } else {
            tile_chance = TileChance{floor: 0.33, wall: 0.33, water: 0.33, sand: 0.0, tree: 0.0};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false};
        }
        biome = Biome {
            biome_name: biome_name,
            tile_chance: tile_chance,
            biome_control: biome_control
        };
        biome
    }
    pub fn default_floor(&self) -> char {
        let floor = match self.biome_name.as_ref() {
            "Cave" => TILE_TYPE.floor,
            "Ocean" => TILE_TYPE.sand,
            "Underlake" => TILE_TYPE.floor,
            "Desert" => TILE_TYPE.sand,
            "Forest" => TILE_TYPE.sand, //dirt next plz
            _ => '.',
        };
        floor
    }
    pub fn default_wall(&self) -> char {
        let wall = match self.biome_name.as_ref() {
            "Cave" => TILE_TYPE.wall,
            "Ocean" => TILE_TYPE.wall, // rock?
            "Underlake" => TILE_TYPE.wall,
            "Desert" => TILE_TYPE.wall, // sandy cliff?
            "Forest" => TILE_TYPE.tree, // tree?
            _ => TILE_TYPE.wall,
        };
        wall
    }
}

fn init(handle: gdnative::init::InitHandle) {
    handle.add_class::<Map>();
}

godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
