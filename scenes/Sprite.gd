extends Sprite
var tile_size = 32 # Sprite size
var key
var mapx
var mapy
var delay = 0.0
var map
var vd_mode = false
var vd_num = 0
var vd_map

func _process(delta):
  if key == "player":
    delay += delta
    if delay > 0.1:
      delay = 0
      if vd_mode:
        get_input_vd() # special movement through points
      else:
        get_input()
  
func is_floor(x, y):
  # Reverse y and x for keys... oops curses damn you!
  var key = str(y) + "x" + str(x)
  if map[key]["c"] == ".":
    return true
  
# This function needs to use voronoi delaunay triangulation
func get_input_vd():
  """move between vd points"""
  if Input.is_action_pressed("up") or Input.is_action_pressed("right"):
    vd_num += 1
  elif Input.is_action_pressed("down") or Input.is_action_pressed("left"):
    vd_num -= 1
  if vd_num < 0:
    vd_num = 0
  if vd_num >= vd_map.size():
    vd_num = vd_map.size()
  update_pos(vd_map["v"+str(vd_num)]["x"],vd_map["v"+str(vd_num)]["y"])
  print(vd_map["v"+str(vd_num)])
  
func get_input():
  if Input.is_action_pressed("ui_up"):
    move_up()
  elif Input.is_action_pressed("ui_down"):
    move_down()
  elif Input.is_action_pressed("ui_left"):
    move_left()
  elif Input.is_action_pressed("ui_right"):
    move_right()

func update_pos(posx, posy):
  mapx = posx
  mapy = posy
  position = Vector2(mapx*tile_size, mapy*tile_size)

func move_left():
  if is_floor(mapx - 1, mapy):
    mapx -= 1
    position = Vector2(mapx * tile_size, mapy * tile_size)

func move_right():
  if is_floor(mapx + 1, mapy):
    mapx += 1
    position = Vector2(mapx * tile_size, mapy * tile_size)
  
func move_up():
  if is_floor(mapx, mapy - 1):
    mapy -= 1
    position = Vector2(mapx * tile_size, mapy * tile_size)
  
func move_down():
  if is_floor(mapx, mapy + 1):
    mapy += 1
    position = Vector2(mapx * tile_size, mapy * tile_size)
  