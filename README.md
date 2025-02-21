# vspreview-rs

<a href="https://raw.githubusercontent.com/quietvoid/vspreview-rs/main/assets/00demo.jpg">
    <img src="https://raw.githubusercontent.com/quietvoid/vspreview-rs/main/assets/00demo.jpg" width="600">
</a>

&nbsp;

Minimal and functional VapourSynth script previewer  
Built on top of [egui](https://github.com/emilk/egui) and [vapoursynth-rs](https://github.com/YaLTeR/vapoursynth-rs)  

&nbsp;

### Dependencies
Requires a VapourSynth installation with support for API 3.6 minimum.  
For the GUI, see [eframe](https://github.com/emilk/egui/tree/master/eframe) dependencies.  

### Building
The minimum Rust version to build `vspreview-rs` is 1.85.0.

`RUSTFLAGS="-C target-cpu=native" cargo build --release`  
Targeting the CPU is highly recommended to get the most performance.

### Running
`cargo run --release -- script.vpy`  
`vspreview-rs script.vpy`  

### GUI

The togglable GUI includes information about the clip as well as interactive controls.  
Also, frame props are easily accessible.

Main parts of the UI:
- A window with the current state, including access to frame props and settings.
- A bottom panel with a slider to change frame quickly, as well as the clip info.
- An error window for VapourSynth messages or errors while rendering.

See more from the [UI documentation](UI.md).

### Config
Using `egui`, the state is persisted across runs.  
Refer to [directories-next](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) docs.

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
- Close: `Escape`, `Q`  
- Show GUI: `I` (toggle)  
- Reload script: `R` 
- Toggle the ICC profile color correction: `C`
- Take a screenshot: `S` (saves to script directory)  
- Copy the current frame number to clipboard: `Ctrl` + `Shift` + `C`  

**Context menu** (right click):  
- Open a new script file
