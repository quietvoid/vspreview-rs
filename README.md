# vspreview-rs
minimal VapourSynth script previewer  

WIP  
Requires OpenGL, probably other stuff. See [eframe](https://github.com/emilk/egui/tree/master/eframe) dependencies.

## Building
`cargo build --release`

## Running
`cargo run --release -- script.vpy`  
`vspreview-rs script.vpy`  

### Config
Using `egui`, the state is persisted across runs.  
Refer to [directories-next](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) docs.

### Keybindings
Seek 1 frame: Right, Left  
Seek 1 second: Down, Up  

Alternative seeking: H, J, K, L  

Change outputs: Num1 to Num0  
Outputs must be from 0-9, and no gaps in between indices

Zoom: Ctrl + Scroll  
Ctrl and +/- for 0.1 zoom increment

Scroll horizontally: Home/End, Shift + Scroll  
Scroll vertically: PageUp/PageDown, Scroll  

Reload script: R, Ctrl+R  

Show OSD info: I (toggle)  
Take a screenshot: S (saves to script directory)  
Close: Escape, Q  
