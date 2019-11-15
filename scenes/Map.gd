extends Node2D

onready var gui = get_node("/root/Main/GUI")
var floor_tile = load("res://scenes/Floor.tscn")
var wall_tile = load("res://scenes/Wall.tscn")
var water_tile = load("res://scenes/Water.tscn")
var player_tile = load("res://scenes/Player.tscn")
var selector_point = load("res://scenes/Selector.tscn")
var star_point = load("res://scenes/Star.tscn")
var canvas_mod = load("res://scenes/CanvasModulate.tscn")
var map = {}
var vd_map = {}
var vd_tile_size = 8
var tile_size = 32 # Sprite size
var player
var rust_map_name = "test101.map"

var map_store = {}
var thread_counter = 0
var mutex
var thread
var semaphore
var file_name
var exit_thread
var thread_busy = false

# Timer to run thread ever 20 seconds
var timer = 0.0
var reset_timer = 2.0


func _ready():
  mutex = Mutex.new()
  thread = Thread.new()
  semaphore = Semaphore.new()
  exit_thread = false
  thread.start(self, "_thread_gen_map")
    
func _process(delta):
  timer += delta
  if timer > reset_timer and not get_thread_busy():
    print("attempting to start thread")
    timer = 0.0
    if len(map_store) > 10:
      print("DEATH TO THREADS", len(map_store))
      exit_thread = true
    semaphore.post()
  update()
  if Input.is_action_pressed("ui_accept"):
    #gui.show()
    $CanvasModulate.hide()
    clear_map()
    #get_tree().reload_current_scene() # only fast way to unload map?
    map = {}
    vd_map = {}
    
func _exit_tree():
    # Set exit condition to true.
    mutex.lock()
    exit_thread = true # Protect with Mutex.
    mutex.unlock()

    # Unblock by posting.
    semaphore.post()

    # Wait until it exits.
    thread.wait_to_finish()
    print("remaining threads: ", get_counter())
 
func get_counter():
    mutex.lock()
    # Copy counter, protect with Mutex.
    var counter_value = thread_counter
    mutex.unlock()
    return counter_value

func get_thread_busy():
    mutex.lock()
    # Copy counter, protect with Mutex.
    var busy = thread_busy
    mutex.unlock()
    return busy

func _thread_gen_map(userdata):
  while true:
    mutex.lock()
    thread_busy = false
    mutex.unlock()
    semaphore.wait() # Wait until posted.
    mutex.lock()
    thread_busy = true
    mutex.unlock()
    mutex.lock()
    var should_exit = exit_thread # Protect with Mutex.
    mutex.unlock()
    
    if should_exit:
      print("exiting thread")
      break
    print("starting thread")
    mutex.lock()
    thread_counter += 1
    mutex.unlock()
    gen_map()
    print("thread finished")
  
  
  
    
# Remove script from all floors and walls!
func update_pos(posx, posy):
  var mapx = posx
  var mapy = posy
  var pos = Vector2(mapx*tile_size, mapy*tile_size)
  return pos
#
#func clear_map_2():
#  var map_node = get_node("/root/Main/Map")
#  var children =  map_node.get_children()
#  if len(children) > 0:
#    for child in children:
#      if child.name != "CanvasModulate":
#        #map_node.remove_child(child) # SLOW AF WTF?
#        call_deferred('_remove_child_by_node', map_node, child, true)
  
  
# What needs to be done is, instantiate Map in godot, queue_free Map in godot
# repeat! Store any data that needs to be kept in main or somewhere else

# Clear map will launch a thread to cleanup old map in the background
func clear_map():
  var cleanup = get_node("/root/Main/Cleanup")
  var map_child = get_node("/root/Main/Map/" + file_name)
  #print("test1:", file_name)
  #print("test2:", get_children())
  #for child in get_children():
  #  print("child: ", child.name)
  #  print("children: ", child.get_children())
  var map_children = map_child.get_children()
  player.hide()
  for child in map_children:
      child.hide()
  $CanvasModulate.hide()
  # Apparently showing and hiding doesn't work in godot, so lets just not use it?!
#  var m = get_node("/root/Main/GUI")
#  m.show()
#  gui.show()
  cleanup.clear_map(file_name)
  play_tiles()
  
func play_vd():
  """show v region connections"""
  gui.hide()
  #gen_map()
  load_map()
  draw_map_points()
  draw_map_lines()
  add_player(true)
  
func play_tiles():
  """show tiles"""
  #gen_map()
  if not load_map():
    return
  gui.hide()
  draw_map_tiles()
  add_player()
  $CanvasModulate.hide()
  #$CanvasModulate.show()
  #get_node("/root/Main/Map/" + file_name + "canvas_mod").show()
  
# Give string tell me if file exists
func exists(f):
  if File.new().file_exists(f):
    return true
  return false
  
#little rewirte to include file checks
func detect_old_maps():
  var map_count= len(map_store)
  var map_path = "resources/maps/"
  var map_name = "f" + str(map_count)
  var full_path = "res://" + map_path + map_name + ".map"
  # [IMPORTANT] Loop through files in directory and find which exist
  # Also use /tmp tmpfs if available
  
  
  
func gen_map():
  var gen_map = get_node("/root/Main/Parent")
  var map_count = len(map_store)
  var map_path = "resources/maps/"
  var map_name = "f" + str(map_count)
  mutex.lock()
  map_store[map_name] = false
  mutex.unlock()
  #gen_map.godot_new_map(map_path + map_name + ".map")
  gen_map.godot_new_biome(map_path + map_name + ".map", "Cave")
  #gen_map.godot_random_biome(map_path + map_name + ".map")
  mutex.lock()
  map_store[map_name] = true
  mutex.unlock()
  print(map_store)
  #gen_map.godot_save_map(rust_map_name)

# Loads the map in var map
func load_map():
  # If there are no maps to load, fail to load
  if len(map_store) < 1:
    print("map store empty... please wait...")
    return false
  var f = File.new()
  #f.open("res://resources/maps/50vc11.map", f.READ)
  for key in map_store.keys():
    if map_store[key]:
      file_name = key
      print("file found: " + key)
      break
  # If you find no valid maps, fail to load
  if file_name == "":
    print("no file found")
    return false
  if not map_store[file_name]:
    print("map is not available")
    return false
  # Let thread know you are using it's dictionary
  mutex.lock()
  map_store[file_name] = false
  mutex.unlock()
  f.open("res://resources/maps/" + file_name + ".map", f.READ)
  var json_string = f.get_as_text()
  var json = JSON.parse(json_string)
  if json.error == OK:
    map = json.result
    extract_vd_points() # extract voronoi regions from map
  else:
    print("error with json")
  return true

# Extract vd_points
func extract_vd_points():
  var regex = RegEx.new()
  regex.compile("v\\d+")
  for key in map.keys():
    var result = regex.search(key)
    if result:
      vd_map[key] = map[key]

func draw_map_points():
  for key in vd_map.keys():
    var s = star_point.instance()
    s.key = key
    s.tile_size = vd_tile_size
    s.update_pos(vd_map[key]['x'], vd_map[key]['y'])
    add_child(s)

# Draw lines between neighbors for vd_map points
func draw_map_lines():
  update()

# Draws the points from map (v-regions)
"""func draw_map_points():
  var regex = RegEx.new()
  regex.compile("v\\d+")
  for key in map.keys():
    var result = regex.search(key)
    if result:
      var s = star_point.instance()
      s.key = key
      s.update_pos(map[key]['x'], map[key]['y'])
      add_child(s)
"""

# Try and avoid queue_free and just edit the same objects
func update_map_tiles():
  load_map()
  if not map:
    return
  
# Draws the tiles from map
func draw_map_tiles():
  var node = Node2D.new()
  #node.set_name(file_name)
  node.name = file_name
  add_child(node)
  if not map:
    return
  # Add canvas light/shadows  
#  var mod = canvas_mod.instance()
#  mod.set_name("canvas_mod")
#  node.add_child(mod)
  for key in map.keys():
    if map[key]['c'] == "#":
      var t = wall_tile.instance()
      #t.key = key
      t.position = update_pos(map[key]['x'], map[key]['y'])
      node.add_child(t)
    elif map[key]['c'] == ".":
      var t = floor_tile.instance()
      #t.key = key
      t.position = update_pos(map[key]['x'], map[key]['y'])
      node.add_child(t)
    elif map[key]['c'] == "~":
      var t = water_tile.instance()
      #t.key = key
      t.position = update_pos(map[key]['x'], map[key]['y'])
      node.add_child(t)
  

# Draws the tiles from map
func draw_map_tiles_old():
  if not map:
    return
  for key in map.keys():
    if map[key]['c'] == "#":
      var t = wall_tile.instance()
      t.key = key
      t.update_pos(map[key]['x'], map[key]['y'])
      add_child(t)
    elif map[key]['c'] == ".":
      var t = floor_tile.instance()
      t.key = key
      t.update_pos(map[key]['x'], map[key]['y'])
      add_child(t)

# Create the player, pass map information to player
func add_player(vd_mode=false):
  var map_child = get_node("/root/Main/Map/" + file_name)
  if vd_mode:
    player = selector_point.instance()
    player.tile_size = vd_tile_size
  else:
    player = player_tile.instance()
  player.key = "player"
  player.map = map
  player.vd_map = vd_map
  player.vd_mode = vd_mode
  player.update_pos(map["mapsize"]["x"] / 2, map["mapsize"]["y"] / 2)
  add_child(player)
  
  
 
  
