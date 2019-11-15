#[macro_use]
extern crate gdnative;

extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate regex;
extern crate flate2;

use gdnative::*;
use rand::Rng;
use std::collections::HashMap;
use std::string::String;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::Path;
use regex::RegexSet;
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
    pub floor: i32, // percentage of map floor
    pub wall: i32,  // percentage of map wall
    pub water: i32 // percentage of map water
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

    // Not used
    #[export]
    unsafe fn godot_save_map(&self, _owner: Node, file_name: GodotString) {
        Map::save_map(&file_name.to_string(), &self, false);
    }

    // Main map generator
    #[export]
    unsafe fn godot_new_map(&self, _owner: Node, file_name: GodotString) {
        let m = Map::new(50, 50, '#', '.', '~', 'P');
        //let m = Map::new_biome(50, 50, String::from("Ocean"));
        Map::save_map(&file_name.to_string(), &m, false);
        //Map::save_map("resources/maps/test101.map", &m, false);
    }

    #[export]
    unsafe fn godot_new_biome(&self, _owner: Node, godot_file_name: GodotString, godot_biome_name: GodotString) {
        // Convert godot string to rust string
        let file_name = godot_file_name.to_string();
        let biome_name = godot_biome_name.to_string();
        let m = Map::new_biome(100, 100, biome_name);
        Map::save_map(&file_name.to_string(), &m, false);
    }

    #[export]
    unsafe fn godot_random_biome(&self, _owner: Node, godot_file_name: GodotString) {
        let mut rng = rand::thread_rng();
        let random_biome = rng.gen_range(0, 4);
        let biome_name;
        if random_biome == 0 {
            biome_name = String::from("Cave");
        } else if random_biome == 1 {
            biome_name = String::from("Ocean");
        } else if random_biome == 2 {
            biome_name = String::from("Underlake");
        } else {
            biome_name = String::from("Terrible_place_to_be");
        }
        let file_name = godot_file_name.to_string();
        let m = Map::new_biome(100, 100, biome_name);
        Map::save_map(&file_name.to_string(), &m, false);
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
        let sizex = 50;
        let sizey = 50;
        let mut rng = rand::thread_rng();
        let number_of_regions = rng.gen_range(sizex+sizey, (sizex+sizey)*2);

        //let mut map_objects = HashMap::new();
        let mut tile_chance;
        let mut biome_control;
        // Prepare the biome data, does NOT enforce tile percentage
        if biome_name == "Cave" {
            tile_chance = TileChance{floor: 50, wall: 45, water: 5};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        } else if biome_name == "Ocean" {
            tile_chance = TileChance{floor: 5, wall: 5, water: 90};
            biome_control = BiomeControl{outer_wall: false, water_edges: true};
        } else if biome_name == "Underlake" {
            tile_chance = TileChance{floor: 20, wall: 20, water: 60};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        } else {
            tile_chance = TileChance{floor: 33, wall: 33, water: 33};
            biome_control = BiomeControl{outer_wall: true, water_edges: true};
        }
        let mut tileset: HashMap<String, Tile>;
        let voronoi_regions: Vec<Tile>;
        // Pass 1: generate voronoi_regions using the TileChance to control biome creation
        voronoi_regions = Map::select_voronoi_regions(sizex, sizey,  &tile_chance, number_of_regions);
        // Pass 2: generate empty tileset
        tileset = Map::empty_tileset(sizex, sizey);
        // Pass 3: convert empty tileset to closest voronoi regions
        tileset = Map::tiles_to_voronoi(tileset, voronoi_regions, number_of_regions); // BROKEN
        // Pass 4: update neighbors of each tile (include corners)
        tileset = Map::update_all_neighbors(sizex, sizey, tileset);
        // Pass 5: make sure all tiles around water are floor
        if biome_control.water_edges {
            tileset = Map::add_water_edges(sizex, sizey, tileset);
        }
        // Pass 6: update wall_borders
        if biome_control.outer_wall {
            tileset = Map::add_wall_borders(sizex, sizey, tileset);
        }
        // Pass X: triangulation (skipping)

        let mapsize = Tile::new(sizey, sizex, '$', Vec::new());
        let player = Tile::new(0, 0, 'p', Vec::new());
        // Additional metadata to save in case another programs needs to know the map size
        tileset.insert(String::from("mapsize"), mapsize);
        tileset.insert(String::from("player"), player);

        // Build Map structure
        let map: Map = Map {
            map_wall: '#',
            map_floor: '.',
            map_water: '~',
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

    fn select_voronoi_regions(sizex: i32, sizey: i32, tile_chance: &TileChance, number_of_regions: i32) -> Vec<Tile> {
        let mut voronoi_regions = Vec::new(); // Vec is duplicated for faster conversion of tiles later
        let mut rng = rand::thread_rng();
        for region in 0..number_of_regions {
            // Select random location for the region
            let x = rng.gen_range(0, sizex);
            let y = rng.gen_range(0, sizey);
            let key = String::from("v") + &region.to_string();
            let tile = Tile::new_random(y, x, &tile_chance, Vec::new());
            //let tile = Tile::new(y, x, '.', Vec::new());
            voronoi_regions.push(tile.clone());
        }
        voronoi_regions.push(Tile::new(sizey/2, sizex/2, TILE_TYPE.floor, Vec::new())); // Player spawn
        voronoi_regions
    }

    // Convert empty tiles in tileset to closest voronoi region type
    fn tiles_to_voronoi (tileset: HashMap<String, Tile>, voronoi_regions: Vec<Tile>, number_of_regions: i32) -> HashMap<String, Tile> {
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
    fn update_side_neighbors () {}

    // Store side neighbors and corner neighbors
    fn update_all_neighbors (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let x = tileset[tile_key].x;
            let y = tileset[tile_key].y;
            if x <= 1 || y <= 1 || x >= sizex-1 || y >= sizey-1 {
                continue;
            }
            let tile_type = tileset[tile_key].c;
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
            let new_tile = Tile::new(y, x, tile_type, neighbors);
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
            if x <= 2 || y <= 2 || x >= sizex-2 || y >= sizey-2 {
                continue;
            }
            let mut tile_type = tileset[tile_key].c;
            let neighbors = tileset[tile_key].neighbors.clone();
            for neighbor_key in neighbors.clone() {
                if tileset[tile_key].c == TILE_TYPE.water && tileset[&neighbor_key].c == TILE_TYPE.wall {
                    tile_type = TILE_TYPE.floor;
                }
            }
            let new_tile = Tile::new(y, x, tile_type, neighbors);
            new_tileset.insert(tile_key.to_string(), new_tile);
        }
        tileset
    }

    // Convert all tiles found at edges of map to wall
    fn add_wall_borders (sizex: i32, sizey: i32, tileset: HashMap<String, Tile>) -> HashMap<String, Tile> {
        let mut new_tileset = HashMap::new();
        for tile_key in tileset.keys() {
            let y = tileset[tile_key].y;
            let x = tileset[tile_key].x;
            let neighbors = tileset[tile_key].neighbors.clone();
            if x == 0 || y == 0 || x == sizex-1 || y == sizey-1 {
                let new_tile = Tile::new(y, x, TILE_TYPE.wall, neighbors);
                new_tileset.insert(tile_key.to_string(), new_tile);
            } else {
                let old_tile = tileset[tile_key].clone();
                new_tileset.insert(tile_key.to_string(), old_tile);
            }
        }
        new_tileset
    }

    // OLD BAD NO LONGER USED
    // Gen map is used to create a variable sized map using voronoi regions (can be very slow) (needs rewrite plz)
    pub fn new(sizey: i32, sizex: i32, map_wall: char, map_floor: char, map_water: char, map_player: char) -> Map {
        // Random numbers
        let mut rng = rand::thread_rng();
        // Create a new set of game_objects for final results
        let mut go = HashMap::new();
        // Generate v-regions based on size of map (# of regions matches maps.py YOU LIED HERE, HALF AS MANY!)
        let mut v_regions = Vec::new();
        let number_of_regions = rng.gen_range((sizey + sizex)/2, (sizey + sizex)*2);
        for v in 0..number_of_regions {
            let y = rng.gen_range(0, sizey);
            let x = rng.gen_range(0, sizex);
            let tile_type = rng.gen_range(0, 5);
            // Generate a string name for v-regions, must survive in game_objects json: v# is key
            let tv = v.to_string();
            let mut vkey = String::from("v");
            vkey.push_str(&tv);
            if tile_type < 2 {
                v_regions.push(Tile::new(y, x, map_wall, Vec::new()));
                go.insert(vkey, Tile::new(y, x, map_wall, Vec::new()));
            } else if tile_type < 4 {
                v_regions.push(Tile::new(y, x, map_floor, Vec::new()));
                go.insert(vkey, Tile::new(y, x, map_floor, Vec::new()));
            } else {
                v_regions.push(Tile::new(y, x, map_water, Vec::new()));
                go.insert(vkey, Tile::new(y, x, map_water, Vec::new()));
            }
        }
        // Add extra region for spawn location
        v_regions.push(Tile::new(sizey/2, sizex/2, map_floor, Vec::new()));
        // Connect vregions through triangulation (update v-region tile closest neighbors)
        for v in 0..number_of_regions {
            let mut closest1 = String::new();
            let mut distance1 = 1000;
            let mut closest2 = String::new();
            let mut distance2 = 1000;
            // Create vkey
            let vkey_num = v.to_string();
            let mut vkey = String::from("v");
            vkey.push_str(&vkey_num);
            // Check each region for closest neighbors
            // Need to enforce angle sizes later...
            for vc in 0..number_of_regions {
                // Skip if self, a points closest point can't be itself
                if v == vc {
                    continue;
                }
                // Create key for closest compare
                let mut vckey = String::from("v");
                vckey.push_str(&vc.to_string());
                // calc distance
                let vcdist = Tile::distance_slow(&go[&vkey], &go[&vckey]);
                // Check if distance is closer, shuffle down the largest distance
                if vcdist < distance1 {
                    if distance1 < distance2 {
                        distance2 = distance1.clone();
                        closest2 = closest1.clone();
                    }
                    distance1 = vcdist.clone();
                    closest1 = vckey.clone();
                } else if vcdist < distance2 {
                    distance2 = distance1.clone();
                    closest2 = closest1.clone();
                }
            }
            // Add two closest neighbors to go map
            let y = go[&vkey].y;
            let x = go[&vkey].x;
            let c = go[&vkey].c;
            let mut neighbors = Vec::new();
            neighbors.push(closest1.clone());
            neighbors.push(closest2.clone());
            let t = Tile::new(y, x, c, neighbors);
            go.insert(t.get_tile_key(), t);
        }
        // Need to do multiple passes before storing in go!
        // You need to modify not only the tile you are working on, BUT ALL THE NEIGHBORS MUST BE UPDATED WITH THIS TILE
        // Maybe we get the 5 closest points and make triangles that force specific angle sizes
        // This could be done by making an additional pass over the neighbors and eliminating any that have bad characteristics
        // This could also lead to a situation where, if a point cannot be used in any valid triangles, we destroy it
        //take a break from this for now, lost motivatio    n
        let mapsize = Tile::new(sizey, sizex, '$', Vec::new());
        let player = Tile::new(0, 0, map_player, Vec::new());
        // Additional metadata to save in case another programs needs to know the map size
        go.insert(String::from("mapsize"), mapsize);
        go.insert(String::from("player"), player);
        // Manually add each v-region Tile
        // Hashmap for storing tiles and map data
        let mut game_objects = HashMap::new();
        // Generate temporary tiles
        for y in 0..sizey {
            for x in 0..sizex {
                let t = Tile::new(y, x, map_floor, Vec::new());
                // update neighbors
                game_objects.insert(t.get_tile_key(), t);
            }
        }
        // Create voronoi regions, convert tiles in game_objects to closest v-region tile type
        // This loop plus the distance calc might be slowest part of gen_map
        for k in game_objects.keys() {
            let mut closest: usize = 0;
            // Calc the closest vregion for this tile, save index in closest
            for i in 0..number_of_regions+1 { // Add +1 extra region for spawn location
                // Distance calc code
                let cur: usize = i as usize;
                let diff = Tile::distance(&v_regions[cur], &game_objects[k]);
                let old_diff = Tile::distance(&v_regions[closest], &game_objects[k]);
                if diff < old_diff {
                    closest = cur;
                }
            }
            // Prepare neighbors
            let y = game_objects[k].y;
            let x = game_objects[k].x;
            let neighbors: Vec<String> = vec![  // Sides
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
            // Make changes to standard voronoi pattern (this is expensive if lots of water!)
            let mut ttype: char = v_regions[closest].c;
            if game_objects[k].y <= 0 || game_objects[k].y >= sizey - 1 || game_objects[k].x <= 0 || game_objects[k].x >= sizex - 1 {
                // Add walls on edges because why not
                ttype = map_wall;
            }
            let t = Tile::new(game_objects[k].y, game_objects[k].x, ttype.clone(), neighbors);
            //let t = Tile {y: game_objects[k].y, x: game_objects[k].x, c: ttype, neighbors: neighbor};
            // Modifying game_objects tiles is a pain, so we make the changes to a mirror data structure called go
            go.insert(k.to_string(), t);
        }

        // Attempt second pass over go to clean up water

        // water edges
        let mut remove_water = HashMap::new();;
        for k in go.keys() {
            let y = go[&k.to_string()].y;
            let x = go[&k.to_string()].x;
            let mut c = go[&k.to_string()].c;
            let n = go[&k.to_string()].neighbors.clone();
            for neighbor_key in go[&k.to_string()].neighbors.clone() {
                if go[&k.to_string()].c == map_water && go[&neighbor_key.to_string()].c == map_wall {
                    c = map_floor;
                }
            }
            let t = Tile::new(y,x,c,n);
            remove_water.insert(k.to_string(), t);
        }

        // remove_water is the new temporary go until I figure out a better way of running passing changes over this map gen...
        // return the modified map data structure
        let map: Map = Map {
            map_wall: '#',
            map_floor: '.',
            map_water: '~',
            map_player: 'p',
            map_game_objects: remove_water
        };
        map
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
    // This function returns a random tile based on the chances provided,
    // calling more than once does NOT increase or decrease the chances of getting a specific tile
    pub fn new_random(y: i32, x: i32, tile_chance: &TileChance, neighbors: Vec<String>) -> Tile {
        // Use the struct without static limitations (slower)
        let tile_vec = tile_chance.vec();
        let mut tile_hash = tile_chance.hash();
        // Calculate sum of chances
        let mut max_tile_chance = tile_chance.sum();
        // This tile
        let mut rng = rand::thread_rng();
        let tile_type = rng.gen_range(0, max_tile_chance);
        let mut tile = '.';
        for value in tile_vec {
            if tile_type < value {
                tile = tile_hash[&value];
            }
        }
        Tile { y: y, x: x, c: tile, neighbors: neighbors }
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

// Small helpful functions for biome creation
impl TileChance {
    // Return struct in hash form, uses static struct names (update if new tile_type is added)
    pub fn hash(&self) -> HashMap<i32,char> {
        // make a HashMap
        let mut tile_hash = HashMap::new();
        tile_hash.insert(self.floor,'.');
        tile_hash.insert(self.wall,'#'); // wall
        tile_hash.insert(self.water, '~');
        tile_hash
    }
    // Return all struct values in a vec, remove 0 values
    pub fn vec(&self) -> Vec<i32> {
        let mut tile_hash = self.hash();
        let mut tile_chance_vec = Vec::new();
        for key in tile_hash.keys() {
            let mut value = *key;
            if value != 0 {
                tile_chance_vec.push(value);
            }
        }
        tile_chance_vec.sort(); // Sort lowest percetages first
        tile_chance_vec
    }
    // Return sum of chance values
    pub fn sum(&self) -> i32 {
        let mut max_tile_chance = 0;
        let mut tile_vec = self.vec();
        for value in tile_vec {
            max_tile_chance += value;
        }
        max_tile_chance
    }
}

fn init(handle: gdnative::init::InitHandle) {
    handle.add_class::<Map>();
}



godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
