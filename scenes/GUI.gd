extends MarginContainer

onready var map = get_node("/root/Main/Map")


func _on_Show_Tiles_pressed():
  map.play_tiles()
  #var Map = get_node("/root/Main/Parent")
  #var poop = Map.poop()
  #print(poop)

func _on_Show_Voronoi_pressed():
  map.play_vd()
