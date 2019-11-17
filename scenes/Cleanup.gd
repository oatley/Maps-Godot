extends Node
var map
var mutex
var thread
var semaphore
var exit_thread
var thread_counter
var thread_busy = false
var timer = 0.0
var reset_timer = 5.0
var clean_map_file_name

func _ready():
  mutex = Mutex.new()
  thread = Thread.new()
  semaphore = Semaphore.new()
  exit_thread = false
  thread.start(self, "_thread_clean_map")
  map = get_node("/root/Main/Map")
 
func clear_map(file_name):
  clean_map_file_name = file_name
  semaphore.post()

func _thread_clean_map(userdata):
  while true:
    # Waiting area for when the thread starts again
    #set_thread_busy(false)
    semaphore.wait() # Wait until posted.
    #set_thread_busy(true)
    
    # Check to see if thread needs to exit
    mutex.lock()
    var should_exit = exit_thread # Protect with Mutex.
    mutex.unlock()
    if should_exit:
      print("[THREAD_cleanup]-> exiting thread")
      break
      
    print("[THREAD_cleanup]-> starting thread CLEAN")
#    mutex.lock()
#    thread_counter += 1
#    mutex.unlock()
    # Do work here:
    var map_child = get_node("/root/Main/Map/" + clean_map_file_name)
    var map_children = map_child.get_children()
    #map.player.queue_free()
    call_deferred("free", map_child)
    print("[THREAD_cleanup]-> map free")
    for child in map_children:
      if child:
        call_deferred("free", child)
    #map_child.queue_free()
    print(map_child, map_child.get_children())
    print("[THREAD_cleanup]-> thread finished")


func _exit_tree():
    # Set exit condition to true.
    mutex.lock()
    exit_thread = true # Protect with Mutex.
    mutex.unlock()

    # Unblock by posting.
    semaphore.post()

    # Wait until it exits.
    thread.wait_to_finish()
    print("[THREAD_cleanup]-> remaining threads: ", get_thread_counter())
   
func get_thread_busy():
  mutex.lock()
  var busy = thread_busy
  mutex.unlock()
  
func set_thread_busy(busy):
  mutex.lock()
  thread_busy = busy
  mutex.unlock()

func get_thread_counter():
  mutex.lock()
  var counter = thread_counter
  mutex.unlock()
    
  
  
  
  
  
  