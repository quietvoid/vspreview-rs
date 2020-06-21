# vspreview-rs
minimal VapourSynth script previewer

WIP  
Requires OpenGL, probably other stuff.

## Building
`cargo build --release`

## Running
`cargo run --release -- script.vpy`

`vspreview-rs script.vpy`

### Keybindings
Seek 1 frame: Right, Left  
Seek 1 second: Down, Up  

Zoom: Ctrl + Scroll

Scroll horizontally: Home/End, Shift + Scroll  
Scroll vertically: PageUp/PageDown, Scroll  

Take a screenshot: S (saves to script directory)  
Close: Escape  