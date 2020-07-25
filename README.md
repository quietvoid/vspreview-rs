# vspreview-rs
minimal VapourSynth script previewer  

WIP  
Requires OpenGL, probably other stuff.  

## Building
`cargo build --release`

## Running
`cargo run --release -- script.vpy`  
`vspreview-rs script.vpy`  

### Config
Using `confy`, automatically saves the config to the user config directory.  

### Keybindings
Seek 1 frame: Right, Left  
Seek 1 second: Down, Up  

Alternative seeking: H, J, K, L  

Zoom: Ctrl + Scroll  

Scroll horizontally: Home/End, Shift + Scroll  
Scroll vertically: PageUp/PageDown, Scroll  

Reload script: F5, R

Show OSD info: I (toggle)  
Take a screenshot: S (saves to script directory)  
Close: Escape, Q  