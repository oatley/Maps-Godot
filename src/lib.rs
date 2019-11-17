#[macro_use]
extern crate gdnative;

extern crate rand;
extern crate serde;
extern crate serde_json;
//extern crate regex;
extern crate flate2;

use gdnative::*;
use rand::Rng;
use std::collections::HashMap;
use std::string::String;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
//use std::path::Path;
//use regex::RegexSet;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

// TODO
// - Rewrite the entire Map::new() function inside Map::godot_new_biome()
// -- Use TileChance to create the biomes with correct tiles
// - Fine tune TileChance to be more accurate or faster
// - Map is cloated
// - Remove angry swearing comments

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tile { // Individual tile data, stored in Map struct HashMap
    pub y: i32,
    pub x: i32,
    pub c: char,
    pub neighbors: Vec<String> // this will store a key to game_objects, for each neighbor tiles
}
pub struct TileChance { // Used to control the biome tiles on map
    pub floor: f32, // percentage of map floor
    pub wall: f32,  // percentage of map wall
    pub water: f32 // percentage of map water
}
pub struct BiomeControl { // Used to control advanced biome manipulation
    pub water_edges: bool, // Activate method add_water_edges(), makes floor around water
    pub outer_wall: bool // Activate method add_border_walls(), add wall around map
}
pub struct TileType { // Static struct to store char for each type_type '.' '#' '~'
    pub floor: char,
    pub wall: char,
    pub water: char
}
static TILE_TYPE: TileType = TileType { // Static struct of TileType, avoid hardcode chars in methods
    floor: '.',
    wall: '#',
    water: '~'
};

#[derive(gdnative::NativeClass)]
#[inherit(gdnative::Node)]
pub struct Map {
    pub map_wall: char,
    pub map_floor: char,
    pub map_water: char,
    pub map_player: char,
    pub map_game_objects: HashMap<String, Tile>
}

#[gdnative::methods]
impl Map {
    // Godot requires this, but I don't need to run anything once... Maybe clear all files in maps dir?
    // This could run prep code to get the directories ready for map files, maybe later
    fn _init(_owner: Node) -> Self {
        let h: HashMap<String, Tile> = HashMap::new();
        let m = Map {map_wall:'0',map_floor:'0',map_water:'~',map_player:'0',map_game_objects:h};
        m
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
        let random_biome = rng.gen_range(0, 3);
        let biome_name;
        if random_biome == 0 {
            biome_name = String::from("Cave");
        } else if random_biome == 1 {
            biome_name = String::from("Ocean");
        } else if random_biome == 2 {
            biome_name = String::from("Underlake");
        } else {
            biome_name = String::from("Cave");
        }
        let file_name = godot_file_name.to_string();
        let m = Map::new_biome(50, 50, biome_name.clone());
        Map::save_map(&file_name.to_string(), &m, false);
        // Return the random biome to godot for logging
        GodotString::from_str(&biome_name)
    }

    // Generate new map of a specific biome
    pub fn new_biome(sizey: i32, sizex: i32, biome_name: String) -> Map {
        //let biome_name = String::from("Ocean");
        // Selecting a biome generates a map created from tiles with specific percentage chance
        // Cave: 50% floor, 45% wall, 5% water
        // Ocean: 85% water, 10% floor, 5% vegetation
        // River: 45% floor, 25% water, 25% wall, 5% bridge
        // TILE_TYPE static struct is used for each different tile

        // How does it work?
        // 1. Initial map gen, store in mut game_objects
        // 2. Split "new" into different "passes" methods
        // 3. Pathfinding cargo object (mostly for drawing lines like roads, rivers, bridges, exits, maybe buildings?)

        // Example:
        // - Gen Map
        // - pass_1-5: biomes
        // - pass_6: add deep/shallow water
        // - pass_7: add roads
        // - pass_8: add bridges over deep water
        // - pass_?: ???

        // Temporary arguments
        //let sizex = 50;
        //let sizey = 50;
        let mut rng = rand::thread_rng();
        let number_of_regions = rng.gen_range(sizex+sizey, (sizex+sizey)*2);

        //let mut map_objects = HashMap::new();
        let tile_chance;
        let biome_control;
        // Prepare the biome data, does NOT enforce tile percentage
        // BUG BUG BUG: if two tiles have same value, one will be ignored... Bug in Tile::new_random()
        if biome_name == "Cave" {
            tile_chance = TileChance{floor: 0.65, wall: 0.3, water: 0.05};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        } else if biome_name == "Ocean" {
            tile_chance = TileChance{floor: 0.05, wall: 0.05, water: 0.9};
            biome_control = BiomeControl{outer_wall: false, water_edges: true};
        } else if biome_name == "Underlake" {
            tile_chance = TileChance{floor: 0.2, wall: 0.2, water: 0.6};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        } else {
            tile_chance = TileChance{floor: 0.33, wall: 0.33, water: 0.33};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        }
        let mut tileset: HashMap<String, Tile>;
        let voronoi_regions: Vec<Tile>;
        // Pass 1: generate voronoi_regions using the TileChance to control biome creation
        voronoi_regions = Map::create_voronoi_regions(sizex, sizey,  &tile_chance, number_of_regions);
        // Pass 2: generate empty tileset
        tileset = Map::empty_tileset(sizex, sizey);
        // Pass 3: convert empty tileset to closest voronoi regions
        tileset = Map::tiles_to_voronoi(tileset, voronoi_regions);
        // Pass 4: update neighbors of each tile (include corners)
        tileset = Map::update_all_neighbors(sizex, sizey, tileset);
        // Pass 5: make sure all tiles around water are floor
        //tileset = Map::add_water_edges(sizex, sizey, tileset);
        if biome_control.water_edges {
            tileset = Map::add_water_edges(sizex, sizey, tileset);
        }
        // Pass 6: update wall_borders
        if biome_control.outer_wall {
            tileset = Map::add_wall_borders(sizex, sizey, tileset);
        }
        // Pass X: triangulation (skipping)

        let mapsize = Tile::new(sizey, sizex, '$', Vec::new());
        let player = Tile::new(0, 0, 'p', Vec::new()); // Legacy
        // Additional metadata to save in case another programs needs to know the map size
        tileset.insert(String::from("mapsize"), mapsize);
        tileset.insert(String::from("player"), player); // Legacy?

        // Build Map structure
        let map: Map = Map {
            map_wall: TILE_TYPE.wall,
            map_floor: TILE_TYPE.floor,
            map_water: TILE_TYPE.water,
            map_player: 'p', // legacy (remove later?)
            map_game_objects: tileset
        };
        map
    }

    // Create empty tileset of a specific size
    fn empty_tileset (sizex: i32, sizey: i32) -> HashMap<String,Tile> {
        let mut tileset = HashMap::new();
        // Generate temporary tiles
        for y in 0..sizey {
            for x in 0..sizex {
                let t = Tile::new(y, x, TILE_TYPE.floor, Vec::new());
                tileset.insert(t.get_tile_key(), t);
            }
        }
        tileset
    }

    fn create_voronoi_regions(sizex: i32, sizey: i32, tile_chance: &TileChance, number_of_regions: i32) -> Vec<Tile> {
        // Get exact number of tiles needed for each type (from TileChance percentage)
        let number_of_floor = (tile_chance.floor * number_of_regions as f32) as i32;
        let number_of_wall = (tile_chance.wall * number_of_regions as f32) as i32;
        let number_of_water = (tile_chance.water * number_of_regions as f32) as i32;
        let mut voronoi_regions = Vec::new();
        // new_voronoi_tiles returns the exact number of tiles requested of the specific type, xy positions are random
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_floor, TILE_TYPE.floor, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_wall, TILE_TYPE.wall, voronoi_regions);
        voronoi_regions = Tile::new_voronoi_tiles(sizex, sizey, number_of_water, TILE_TYPE.water, voronoi_regions);
        voronoi_regions.push(Tile::new(sizey/2, sizex/2, TILE_TYPE.floor, Vec::new())); // Player spawn
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
            let new_tile = Tile::new(tileset[tile_key].y.clone(), tileset[tile_key].x.clone(), voronoi_regions[closest_region].c.clone() , Vec::new());
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Only store side neighbors
    //fn update_side_neighbors () {}

    // Store side neighbors and corner neighbors
    fn update_all_neighbors (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            let tile_type = tileset[tile_key].c;
            let new_tile;
            if x <= 1 || y <= 1 || x >= sizex-1 || y >= sizey-1 {
                new_tile = Tile::new(y, x, tile_type, Vec::new());
            } else {
                let neighbors: Vec<String> = vec![
                    // Sides
                    (y + 1).to_string() + "x" + &(x).to_string(),
                    (y - 1).to_string() + "x" + &(x).to_string(),
                    (y).to_string() + "x" + &(x + 1).to_string(),
                    (y).to_string() + "x" + &(x - 1).to_string(),
                    // Corners
                    (y + 1).to_string() + "x" + &(x + 1).to_string(),
                    (y - 1).to_string() + "x" + &(x - 1).to_string(),
                    (y + 1).to_string() + "x" + &(x - 1).to_string(),
                    (y - 1).to_string() + "x" + &(x + 1).to_string()
                ];
                new_tile = Tile::new(y, x, tile_type, neighbors);
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Change all water tiles touching walls into floor (more walkable space)
    fn add_water_edges (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let y = tileset[tile_key].y;
            let x = tileset[tile_key].x;
            let mut tile_type = tileset[tile_key].c;
            let neighbors = tileset[tile_key].neighbors.clone();
            let new_tile;
            if x <= 2 || y <= 2 || x >= sizex-2 || y >= sizey-2 {
                new_tile = Tile::new(y, x, tile_type, neighbors);
            } else {
                for neighbor_key in neighbors.clone() {
                    if tileset[tile_key].c == TILE_TYPE.water && tileset[&neighbor_key].c == TILE_TYPE.wall {
                        tile_type = TILE_TYPE.floor;
                    }
                }
                new_tile = Tile::new(y, x, tile_type, neighbors);
            }
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        new_tileset
    }

    // Convert all tiles found at edges of map to wall
    fn add_wall_borders (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let y = tileset[tile_key].y;
            let x = tileset[tile_key].x;
            let neighbors = tileset[tile_key].neighbors.clone();
            let new_tile;
            if x == 0 || y == 0 || x == sizex-1 || y == sizey-1 {
                new_tile = Tile::new(y, x, TILE_TYPE.wall, neighbors);
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
        let game_objects: HashMap<String, Tile> = serde_json::from_str(&s).unwrap();
        let map: Map = Map {
            map_wall: '#',
            map_floor: '.',
            map_water: '~',
            map_player: 'p',
            map_game_objects: game_objects
        };
        map
    }

    // Serialize hashmap into string, open a file for writing, write to file with compressed bufwriter
    pub fn save_map (filename: &str, map: &Map, compression: bool) {
        let serialized = serde_json::to_string(&map.map_game_objects).unwrap();
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
    pub fn new(y: i32, x: i32, c: char, neighbors: Vec<String>) -> Tile {
        Tile { y: y, x: x, c: c, neighbors: neighbors }
    }

    // Return new vec tiles, random xy positions, set specific tile type
    pub fn new_voronoi_tiles(sizex: i32, sizey: i32, number_of_tiles: i32, tile_type: char, mut voronoi_regions: Vec<Tile>) -> Vec<Tile> {
        let mut rng = rand::thread_rng();
        let mut tiles_remaining = number_of_tiles;
        while tiles_remaining > 0 {
            tiles_remaining -= 1;
            voronoi_regions.push(
                Tile::new(
                    rng.gen_range(0, sizey),
                    rng.gen_range(0, sizex),
                    tile_type,
                    Vec::new()
                )
            );
        }
        voronoi_regions
    }
    // Create new tile key string, xy coordinate with separator
    pub fn get_tile_key(&self) -> String {
        let ty = &self.y.to_string();
        let tx = &self.x.to_string();
        let sep = String::from("x");
        let mut s = String::new();
        s.push_str(&ty);
        s.push_str(&sep);
        s.push_str(&tx);
        s
    }
    // Calculate distance between tiles for v-regions in gen_map
    pub fn distance(v: &Tile, t: &Tile) -> i32 {
        let distance = (v.y - t.y).abs() + (v.x - t.x).abs();
        distance
    }
    // Calculate distance between tiles for v-regions in gen_map
    pub fn distance_slow(v: &Tile, t: &Tile) -> i32 {
        let y = (v.y - t.y).pow(2) as f64;
        let x = (v.x - t.x).pow(2) as f64;
        let distance = (y + x).sqrt() as i32;
        distance
    }
}

fn init(handle: gdnative::init::InitHandle) {
    handle.add_class::<Map>();
}



godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
