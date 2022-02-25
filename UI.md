The UI has two parts (currently).  
Both can be displayed by toggling with the `I` key.

## State Window

![State window](/assets/01gui.jpg?raw=true "State window")

**Controls**

The following controls are available in window:
- **Output selector**: Change output by selecting from the list.
- **Zoom factor**: Slider/input to adjust the zoom.
- **Translate**: Adjust the image translation.
    - Can only be used when the image does not already fit in the window.

**Frame props**

The frame props default to the converted RGB24 clip for display.  
Original frame props require an extra frame request, so they can be obtained on demand.  

- **Supported frame props**:
    - **Frame type**, `_PictType`.
    - **Color range**, `_ColorRange`.
    - **Chroma location** (only in original props), `_ChromaLocation`.
    - **Primaries** (`_Primaries`), **Matrix** (`_Matrix`), **Transfer** (`_Transfer`).
    - If the frame is a scene cut, `_SceneChangePrev`.
    - **HDR10**/**ST2086** metadata, from `ffms2`.
        - Can be clicked to copy the corresponding `x265` CLI settings.
    - If the frame carries **Dolby Vision** RPU metadata, from `ffms2`.
    - **CAMBI** score, from [akarin.Cambi](https://github.com/AkarinVS/vapoursynth-plugin).

**Preferences**

![Preferences](/assets/03prefs.jpg?raw=true "Preferences")

- **Resizer**: The VapourSynth resizer used to convert to RGB24.
- **Dithering**: whether to add additionnal dithering when converting.
- **Upscale to the window**: can be used to upscale the frame to fit in the window.
    - Useful when the clip is lower resolution than the window.
- **Zoom multiplier**: Multiplies the zoom factor by this value instead of incrementing by 1.0.
- **Scroll multiplier**: Mutliplies the pixels translated on wheel scroll.
    - Can be used to translate faster or slower.
- **Canvas margin**: Padding to add around the image.

&nbsp;

## Bottom Panel

![Bottom panel](/assets/02clipinfo.jpg?raw=true "Bottom panel")

Provides a slider to seek through frames, as well as an input box to enter a specific frame.  
Various informations about the clip.  
