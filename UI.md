The UI has two parts (currently).

The state window  
![State window](/assets/01gui.jpg?raw=true "State window")

A bottom panel  
![Bottom panel](/assets/02clipinfo.jpg?raw=true "Bottom panel")

The frame props default to the converted RGB24 clip for display.  
Original frame props require an extra frame request, so they can be obtained on demand.  

Supported frame props:
- Frame type (`_PictType`)
- Color range (`_ColorRange`)
- Chroma location (only in original props) (`_ChromaLocation`)
- Primaries (`_Primaries`), Matrix (`_Matrix`), Transfer (`_Transfer`)
- If the frame is a scene cut (`_SceneChangePrev`)
- `HDR10`/`ST2086` metadata, from `ffms2`
    - Can be clicked to copy the corresponding `x265` CLI settings
- If the frame carries `Dolby Vision` RPU metadata, from `ffms2`
- CAMBI score, from [akarin.Cambi](https://github.com/AkarinVS/vapoursynth-plugin)

Eventually, the UI will allow modifying the previewer settings.  
Maybe more.
