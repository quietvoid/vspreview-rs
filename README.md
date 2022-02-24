# vspreview-rs
Minimal and functional VapourSynth script previewer  

## Dependencies
See [eframe](https://github.com/emilk/egui/tree/master/eframe) dependencies.

## Building
`RUSTFLAGS="-C target-cpu=native" cargo build --release`  
Targeting the CPU is highly recommended to get the most performance.

### Running
`cargo run --release -- script.vpy`  
`vspreview-rs script.vpy`  

### Keybindings

**Moving around the image/clip**:  
- Seek 1 frame: `Right`, `Left`  
- Seek 1 second: `Down`, `Up`  
- Alternative seeking: `H`, `J`, `K`, `L`  
- Change outputs: `Num1` to `Num0`  
    - Outputs must be from 0-9
- Zoom: `Ctrl` + **Scroll wheel**  
    - `Ctrl` + `Up`/`Down` for 0.1 zoom increments  
- Scroll horizontally: `Home`/`End` or `Shift` + **Scroll wheel**  
- Scroll vertically: `PageUp`/`PageDown`, **Scroll wheel**  

**Misc**:  
- Reload script: `R` 
- Show GUI: `I` (toggle)  
- Take a screenshot: `S` (saves to script directory)  
- Close: `Escape`, `Q`  

### Config
Using `egui`, the state is persisted across runs.  
Refer to [directories-next](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) docs.
