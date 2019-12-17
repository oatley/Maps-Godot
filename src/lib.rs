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

// Priority Todo:
// - Compile a library for use with godot, this library will not be updated, must make basic maps


// To do:
// - After refactor and crate separation: Javascript/webassembly front end map viewer using godot tiles
// - Might be good to separate into multiple crates soon
// - Store biome control and tile chance inside Map (probably not)
// - BiomeControl is getting bulky! (anything to do? probably not)
// - combine biome control and tile chance into single function or creation (maybe)
// - Map can be restructured for simplicity and security: priv Map, pub GodotMap, priv Save/Load/CompressMap (yes please refactor)
// -- Separating map into the generation of the tileset, godot interface, and extra tools for load/save/compress (next refactor)
// - PathMap could export to json... (does it matter?)
// -- This might actually be required for pre-computed paths!!!
// - Exits! Exits can be added to the side of a map, to make maps go infinite (Priority)
// -- The idea is that you generate multiple maps in a grid with connections on specific tiles
// - Roads: connect roads to exits(BiomeControl bool) (Priority)
// - HPA*: think about or research what kind of data structure is simple enough to store abstracted map data (not yet)
// -- I think the most important change or addition is the ability to store a separate optional path for each abstracted PathTile?
// -- It would be useful for path finding across a grid of maps, instead of just tiles too (if it isn't too hard)
// -- Would require: maps that are connected (easy), pre computed path between exits (needs file storage)(hard), store map data in PathTile(?)
// - Maze Biome
// - City Biome
// - Quad-Tree stored world, through a Quad-Sphere or basic cube initially


#[derive(Clone)]
pub struct PathTile {
    pub x: i32,
    pub y: i32,
    pub g: i32,
    pub h: i32,
    pub f: i32,
    pub parent: String,
    pub neighbors: Vec<String>
}

// Used to access PathTile information
pub struct PathMap {
    pub path_tiles: HashMap<String, PathTile>
}

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
    pub sparse_trees: bool,
    pub roads: bool,
    pub exit_roads: bool,
    pub exits: bool
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
    pub sand: char,
    pub tree: char,
    pub road: char,
    pub exit: char
}
static TILE_TYPE: TileType = TileType { // Static struct of TileType, avoid hardcode chars in methods
    floor: '.',
    wall: '#',
    water: '~',
    sand: ',',
    tree: 't',
    road: '.', // Same as floor for testing
    exit: '/'
};

#[derive(gdnative::NativeClass)]
#[inherit(gdnative::Node)]
pub struct Map {
    pub world_x: i32,
    pub world_y: i32,
    pub world_z: i32,
    pub tileset: HashMap<String, Tile>
}

// New world structs (this maybe should NOT be a xyz grid) (is there a better way to do this) (research world generation)
// Stores map_file_name, map_position_on_world, map_biome, connected_map_neighbors
pub struct World {
    pub world_name: String, // Directory name to store maps
    pub size_x: i32, // Max size 5 mean -5 .. 0 .. 5 (inclusive)
    pub size_y: i32, // Allows the manipulation of the world shape
    pub size_z: i32, // Use z to make a giant tower
    pub directory: String,
    pub compress_maps: bool,
    pub maps: HashMap<String, String> // location(xyz) and file_name
}

// Save world in a directory
// world_name/world_name.json
// world_name/maps/map_name.map

impl World {
    fn new (world_name: String, size_x: i32, size_y: i32, size_z: i32) -> World {
        let directory = String::from("/tmp/worlds/") + &world_name;
        let _dir = match fs::create_dir_all(&directory) {
            Ok(_dir) => _dir,
            Err(_e) => (),
        };
        let mut maps: HashMap<String, String> = HashMap::new();
        maps.insert(String::from("world_name"), world_name.to_string());
        maps.insert(String::from("size_x"), size_x.to_string());
        maps.insert(String::from("size_y"), size_y.to_string());
        maps.insert(String::from("size_z"), size_z.to_string());
        maps.insert(String::from("directory"), directory.to_string());
        World {world_name: world_name, size_x: size_x, size_y: size_y, size_z: size_z, directory: directory, compress_maps: false, maps: maps}
    }
    // Test a cube 3x3x3 world
    pub fn new_world_test() {
        let mut world = World::new(String::from("meow"), 3, 3, 3);
        let mut maps = HashMap::new();
        maps.insert(String::from("world_name"), world.world_name.to_string());
        maps.insert(String::from("size_x"), world.size_x.to_string());
        maps.insert(String::from("size_y"), world.size_y.to_string());
        maps.insert(String::from("size_z"), world.size_z.to_string());
        maps.insert(String::from("directory"), world.directory.to_string());
        for x in 0..world.size_x {
            for y in 0..world.size_x {
                for z in 0..world.size_x {
                    let map = Map::new_biome(50, 50, Map::random_biome());
                    let map_name = World::give_map_name(x, y, z);
                    let mut file_path = world.directory.to_string();
                    file_path.push_str("/");
                    file_path.push_str(&map_name);
                    file_path.push_str(".map");
                    Map::save_map(&file_path, &map, false);
                    maps.insert(map_name.to_string(), file_path.to_string());
                }
            }
        }
        world.maps = maps;
        World::save_world(&world, false);
    }
    fn give_map_name(x: i32, y: i32, z: i32) -> String {
        let mut file_name = String::from("x");
        file_name.push_str(&x.to_string());
        file_name.push_str("y");
        file_name.push_str(&y.to_string());
        file_name.push_str("z");
        file_name.push_str(&z.to_string());
        file_name
    }
    fn add_map (file_name: String, x: i32, y: i32, z: i32) {

    }

    fn get_world_path(world_name: String) -> String {
        let mut world_path = String::from("/tmp/worlds/");
        world_path.push_str(&world_name);
        world_path.push_str("/");
        world_path.push_str(&world_name);
        world_path.push_str(".world");
        world_path
    }
    fn load_world (world_name: &str) -> World {
            let world_path = World::get_world_path(world_name.to_string());
            let mut f = File::open(world_path).expect("Unable to open file");
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            let maps: HashMap<String, String> = serde_json::from_str(&s).unwrap();
            let size_x = maps["size_x"].parse::<i32>().unwrap().clone();
            let size_y = maps["size_y"].parse::<i32>().unwrap().clone();
            let size_z = maps["size_z"].parse::<i32>().unwrap().clone();
            let world = World::new(maps["world_name"].clone(), size_x, size_y, size_z);
            world
    }
    // Opens a file for reading to decompress, deserialize, and store as hashmap
    // pub fn load_world(file_name: &str) -> World {
    //     let mut f = File::open(filename).expect("Unable to open file");
    //     let mut s = String::new();
    //     f.read_to_string(&mut s).unwrap();
    //     let tileset: HashMap<String, Tile> = serde_json::from_str(&s).unwrap();
    //     let map = Map::new(tileset);
    //     map
    // }
    //
    // // Serialize hashmap into string, open a file for writing, write to file with compressed bufwriter
    pub fn save_world (world: &World, compression: bool) {
        let serialized = serde_json::to_string(&world.maps).unwrap();
        let world_path = World::get_world_path(world.world_name.to_string());
        let f = File::create(world_path).expect("Unable to create file");
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
}


#[gdnative::methods]
impl Map {
    pub fn test() { // debug testing only
        Map::prep();
        let m = Map::new_biome(50, 50, String::from("Forest"));
        Map::save_map("/tmp/maps/test101.map", &m, false);
    }
    // Build Map structure
    fn new(tileset: HashMap<String, Tile>) -> Map {
        let map: Map = Map {
            world_x: 0,
            world_y: 0,
            world_z: 0,
            tileset: tileset
        };
        map
    }
    // Make new directory, don't error if exists
    fn prep() {
        let _dir = match fs::create_dir_all("/tmp/maps") {
            Ok(_dir) => _dir,
            Err(_e) => (),
        };
    }
    // Make required directories, give placeholder object to godot
    fn _init(_owner: Node) -> Self {
        Map::prep();
        let tileset = HashMap::new();
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
        let biome_name = Map::random_biome();
        let file_name = godot_file_name.to_string();
        let m = Map::new_biome(50, 50, biome_name.clone());
        Map::save_map(&file_name.to_string(), &m, false);
        // Return the random biome to godot for logging
        GodotString::from_str(&biome_name)
    }

    pub fn random_biome() -> String {
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
        biome_name
    }

    #[export] // Specify a map file to read, and a start_tile and end_tile, return path between
    pub fn godot_path_find(&self, _owner: Node, godot_file_name: GodotString, start_tile: GodotString, end_tile: GodotString) -> StringArray {
        // Load map file
        let file_name = godot_file_name.to_string();
        let map = Map::load_map(&file_name, false);
        // Create new PathMap overlay (copy of Map but with cost/parent info)
        let mut path_map = PathMap::new(map.tileset["mapsize"].x, map.tileset["mapsize"].y, &map.tileset);
        // Get the path in Vec<String> (where string is tile keys)
        let mut path = PathMap::find_path(start_tile.to_string(), end_tile.to_string(), path_map.path_tiles, &map.tileset);
        // Convert to Godot StringArray, and return
        let mut godot_array: StringArray = StringArray::new();
        for tile in path {
            godot_array.push(&GodotString::from_str(&tile))
        }
        godot_array
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
        tileset = Map::tiles_to_voronoi(voronoi_regions, tileset);
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
        // Pass 7: Draw a road
        //if biome.biome_control.roads {
            // This is debug/testing only (remove completely later)
            //tileset = Map::draw_road(sizex, sizey, "25x25".to_string(), "15x15".to_string(), tileset);
        //}
        // Pass 8: Exits (for infinitely connected maps)
        if biome.biome_control.exits {
            //tileset = Map::add_map_exits();
        }

        // Pass FINAL: update wall_borders
        if biome.biome_control.outer_wall {
            tileset = Map::add_wall_borders(sizex, sizey, &biome, tileset);
        }
        // Pass X: triangulation (skipping)
        // Pass X: pathfinding

        // Sneaky secret tiles for map_size and default tiles for each biome
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

    // Map needs to know it's position in a map grid (aka a world with a world size?)
    // Could a Map be used to abstract an entire world? (no z-axis is big issue) (this is a bad idea)
    // Do I need to create a world before doing infinite map? (I think so... damn)
    fn add_map_exit(exit_type: String, x: i32, y: i32, mut tileset: HashMap<String,Tile>) -> HashMap<String,Tile> {
        let mut exit_key = String::from("exit_");
        exit_key.push_str(&exit_type);
        let exit_tile = Tile::new(x, y, 'x', Vec::new());
        tileset.insert(exit_key, exit_tile);
        tileset
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
    // (enum?)
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
    fn tiles_to_voronoi (voronoi_regions: Vec<Tile>, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
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

    //
    pub fn draw_road(sizex: i32, sizey: i32, start_tile: String, end_tile: String, mut tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut path_map = PathMap::new(sizex, sizey, &tileset);
        let path = PathMap::find_path(start_tile, end_tile, path_map.path_tiles, &tileset);
        for tile in path {
            let new_tile = Tile::new(tileset[&tile].x, tileset[&tile].y, TILE_TYPE.road, tileset[&tile].neighbors.clone());
            tileset.insert(tile.to_string(), new_tile);
        }
        tileset
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
    // Estimate distance between tiles for heuristic
    pub fn heuristic_distance(v: &Tile, t: &Tile) -> i32 {
        let x = (v.x - t.x).pow(2) as f64;
        let y = (v.y - t.y).pow(2) as f64;
        let distance = (x + y) as i32;
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
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false,
                                        roads: false, exit_roads: false, exits: false};
        } else if biome_name == "Ocean" {
            tile_chance = TileChance{floor: 0.0, wall: 0.05, water: 0.7, sand: 0.15, tree: 0.1};
            biome_control = BiomeControl{outer_wall: false, water_edges: true, sparse_trees: true,
                                        roads: false, exit_roads: false, exits: false};
        } else if biome_name == "Underlake" {
            tile_chance = TileChance{floor: 0.2, wall: 0.2, water: 0.6, sand: 0.0, tree: 0.0};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false,
                                            roads: false, exit_roads: false, exits: false};
        } else if biome_name == "Desert" {
            tile_chance = TileChance{floor: 0.0, wall: 0.2, water: 0.15, sand: 0.5, tree: 0.15};
            biome_control = BiomeControl{outer_wall: false, water_edges: true, sparse_trees: true,
                                        roads: false, exit_roads: false, exits: false};
        } else if biome_name == "Forest" {
            tile_chance = TileChance{floor: 0.0, wall: 0.2, water: 0.2, sand: 0.2, tree: 0.4};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: true,
                                        roads: false, exit_roads: false, exits: false};
        } else {
            tile_chance = TileChance{floor: 0.33, wall: 0.33, water: 0.33, sand: 0.0, tree: 0.0};
            biome_control = BiomeControl{outer_wall: true, water_edges: true, sparse_trees: false,
                                        roads: false, exit_roads: false, exits: false};
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

// Structure copy of the entire map, used to pathfind
impl PathMap {
    // Create new PathMap, path_tiles is a copy of tileset with costs and parent data (could abstract HPA* later?)
    pub fn new(map_size_x: i32, map_size_y: i32, tileset: &HashMap<String, Tile>) -> PathMap {
        let mut path_tiles: HashMap<String, PathTile> = HashMap::new();
        for tile in tileset.values() {
            let mut path_tile = PathTile::new(tile.x, tile.y, map_size_x, map_size_y);
            path_tiles.insert(path_tile.get_tile_key(), path_tile);
        }
        PathMap {path_tiles: path_tiles}
    }

    // Temporary function for pathfinding through usable tiles (might need to path through walls for map-gen)
    // Might need to be an enum or more customizable (build bridges over water, tunnels through mountains, etc)
    pub fn is_walkable(tile_type: char) -> bool {
        let walkable = match tile_type {
            '.' => true,
            ',' => true,
            '#' => true,
            't' => true,
            '~' => true,
            _   => true
        };
        walkable
    }

    // is_walkable could be an enum/struct/something provided to this (control which tiles are walkable on a per path basis)
    // This is a little hard to read, maybe calculating costs can be shrunk down (separate method for costs)
    // A* pathfinding -> returns the shortest_path between two tiles using A* (slow) (goes through walls)
    pub fn find_path(start_node: String, end_node: String, mut path_tiles: HashMap<String, PathTile>, tileset: &HashMap<String, Tile>) -> Vec<String> {
        let mut open_list: Vec<String> = Vec::new();
        let mut closed_list: Vec<String> = Vec::new();
        let mut shortest_path: Vec<String> = Vec::new();
        let mut exit: bool = false;
        let mut lowest_f_cost_node: String = String::from("");
        let mut current_tile;
        open_list.push(start_node.clone());
        while ! exit {
            // Find lowest f cost in open list
            for current in 0..open_list.len() {
                if lowest_f_cost_node == "" {
                    // Calculate costs for starting node, update tile in path_tiles
                    lowest_f_cost_node = path_tiles[&open_list[current]].get_tile_key();
                    let parent = start_node.to_string();
                    let g = Tile::distance(&tileset[&start_node], &tileset[&end_node]);
                    let h = Tile::heuristic_distance(&tileset[&start_node], &tileset[&end_node]);
                    let f = g + h;
                    path_tiles.insert(start_node.clone(), path_tiles[&start_node].tile_update(g, h, f, parent));
                } else if path_tiles[&open_list[current]].f < path_tiles[&lowest_f_cost_node].f {
                    lowest_f_cost_node = path_tiles[&open_list[current]].get_tile_key();
                }
            }
            // Set the lowest_f_cost_node to the current_tile, remove from open_list, add to closed_list
            open_list.retain(|tile_key| tile_key.to_string() != lowest_f_cost_node);
            closed_list.push(path_tiles[&lowest_f_cost_node].get_tile_key());
            current_tile = lowest_f_cost_node.to_string();
            // Search all neighbors to current_tile for destination, calculate new costs
            for neighbor_key in path_tiles[&current_tile].neighbors.clone() {
                // If tile is NOT walkable (skip)
                if ! PathMap::is_walkable(tileset[&neighbor_key].c) {
                    continue;
                }
                // If tile is in the closed list (skip)
                if closed_list.iter().any(|x| x == &neighbor_key) {
                    continue;
                }
                // If tile is NOT on the open list, add it to the open list
                if ! open_list.iter().any(|x| x == &neighbor_key) {
                    // Calculate costs for starting node, update tile in path_tiles
                    open_list.push(neighbor_key.to_string());
                    let parent = current_tile.to_string();
                    let g = Tile::distance(&tileset[&current_tile], &tileset[&neighbor_key]);
                    let h = Tile::heuristic_distance(&tileset[&neighbor_key], &tileset[&end_node]);
                    let f = g + h;
                    path_tiles.insert(neighbor_key.clone(), path_tiles[&neighbor_key].tile_update(g, h, f, parent));
                } else { // Tile IS on the open list, check if this path's g-cost is lower than the previous cost
                    let new_g_cost = Tile::distance(&tileset[&current_tile], &tileset[&neighbor_key]);
                    // if this new path's g-cost is lower, calculate new costs and update path_tiles
                    if new_g_cost < path_tiles[&neighbor_key].g {
                        let parent = current_tile.to_string();
                        let g = Tile::distance(&tileset[&current_tile], &tileset[&neighbor_key]);
                        let h = Tile::heuristic_distance(&tileset[&neighbor_key], &tileset[&end_node]);
                        let f = g + h;
                        path_tiles.insert(neighbor_key.clone(), path_tiles[&neighbor_key].tile_update(g, h, f, parent));
                    }
                }
            } // end of current_tile.neighbors
            // Path success between start and end, exit main loop, return path Vec<string> of tile keys
            if closed_list.iter().any(|x| x == &end_node) {
                shortest_path = PathMap::trace_path(start_node.clone(), end_node.clone(), &path_tiles);
                exit = true;
                break;
            // Path failed to connect start and end nodes (possible if no walkable route)
            } else if open_list.len() == 0 && ! closed_list.iter().any(|x| x == &end_node) {
                shortest_path = Vec::new();
                exit = true;
                break;
            }
        } // end of while exit
        shortest_path
    }

    // Used once find_path gets the end_node in closed_list, traces parents back to start_node
    pub fn trace_path (start_node: String, end_node: String, path_tiles: &HashMap<String, PathTile>) -> Vec<String> {
        let mut current_node = end_node.to_string();
        let mut shortest_path: Vec<String> = Vec::new();
        while current_node != start_node {
            shortest_path.push(current_node.clone());
            current_node = path_tiles[&current_node].parent.clone();
        }
        shortest_path.push(start_node.clone());
        shortest_path
    }

}

// Individual tiles for pathfinding
impl PathTile {
    pub fn new(x: i32, y: i32, map_size_x: i32, map_size_y: i32) -> PathTile {
        let mut path_tile = PathTile {
            x: x,
            y: y,
            g: 0,
            h: 0,
            f: 0,
            parent: String::from(""),
            neighbors: PathTile::get_neighbors(x, y, map_size_x, map_size_y)
        };
        path_tile
    }

    // Generate neighbor keys
    pub fn get_neighbors(x: i32, y: i32, map_size_x: i32, map_size_y: i32) -> Vec<String> {
        let mut neighbors: Vec<String> = Vec::new();
        if x+1 < map_size_x {
            let right_side = (x + 1).to_string() + "x" + &(y).to_string();
            neighbors.push(right_side);
        }
        if x-1 > 0 {
            let left_side = (x - 1).to_string() + "x" + &(y).to_string();
            neighbors.push(left_side);
        }
        if y+1 < map_size_y {
            let bottom_side = (x).to_string() + "x" + &(y + 1).to_string();
            neighbors.push(bottom_side);
        }
        if y-1 > 0 {
            let top_side = (x).to_string() + "x" + &(y - 1).to_string();
            neighbors.push(top_side);
        }
        neighbors
    }
    // Maybe calculate costs here?
    pub fn tile_update(&self, g: i32, h: i32, f: i32, parent: String) -> PathTile {
        let new_path_tile = PathTile {
            x: self.x.clone(),
            y: self.y.clone(),
            g: g,
            h: h,
            f: f,
            parent: parent,
            neighbors: self.neighbors.clone()
        };
        new_path_tile
    }
    // Do we need two different update functions?
    pub fn change_parent(&self, parent: String) -> PathTile {
        let new_path_tile = PathTile {
            x: self.x.clone(),
            y: self.y.clone(),
            g: self.g.clone(),
            h: self.h.clone(),
            f: self.f.clone(),
            parent: parent,
            neighbors: self.neighbors.clone()
        };
        new_path_tile
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
}

fn init(handle: gdnative::init::InitHandle) {
    handle.add_class::<Map>();
}

godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
