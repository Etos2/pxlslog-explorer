# PxlsLog-Explorer
A simple command line utility program to filter logs and generate timelapses for [pxls.space](https://pxls.space/).
Designed for pxls.space users to explore their personal logs or to be utilised in mass analysis.
Forgot where the logs are? They're [here](https://pxls.space/x/logs/). Your personal hashes are located on your profile page.

### Current features:
- Simple program settings
  - Disable overwritting existing files
- Filter entries to file (Defaults to STDOUT, be warned)
  - Via provided date (Format: %Y-%m-%dT%H:%M:%S%.f)
  - Via colour index
  - Via region (Format: x1,y1,x2,y2)
  - Via actions (place, undo, overwrite, rollback, rollback-undo, nuke)
  - Via user hash


## Usage
To get on track, seek the help argument.
This will list basic arguments and display available subcommands.
Alternatively, use on a subcommand to produce arguments for the subcommand.

```
pxlslog-explorer.exe --help
pxlslog-explorer.exe filter --help
pxlslog-explorer.exe render --help
```

The filter subcommand is quite simple, follow the syntax hints for tricky filters such as "--after", "--before" and "--region".
Be wary with many filters applied, it can be quite messy:

```
// Print straight to STDOUT
pxlslog-explorer.exe filter pixels_cXX.sanit.log

// Write to mypixels_cXX.log when color == 5
pxlslog-explorer.exe filter --color 5 pixels_cXX.sanit.log mypixels_cXX.log

// Write to mypixels2_cXX.log when color == 5, after == "2021-04-12T23:56:04" and user = "insert hash here"
pxlslog-explorer.exe filter --color 5 --after 2021-04-12T23:56:04 --user (insert hash here) pixels_cXX.sanit.log mypixels2_cXX.log
```

The render subcommand accepts a log file and produces frames in the desired format. However it can also produce a single complete frame.
A palette can also be provided, either from raw json found [here](https://pxls.space/info) or palette files found [here](https://pxls.space/x/palette) [.gpl, .aco, .csv, .txt (paint.NET)].
See the [image](https://crates.io/crates/image) crate for supported image formats.

```
// Using background as source, produce a frame every 5 minutes in the PNG format
pxlslog-explorer.exe render -s logs/pixels_cXX.sanit.log -d out/cXX.png --bg canvas/cXX.png --step 300000
// Or, produce a single frame
pxlslog-explorer.exe render -s logs/pixels_cXX.sanit.log -d out/cXX.png --bg canvas/cXX.png --screenshot
// You can also skip frames, and provide a custom palette.
pxlslog-explorer.exe render -s logs/pixels_cXX.sanit.log -d out/cXX.png --bg canvas/cXX.png --screenshot --palette palette/p10.gpl
```

Additionally, frames can be piped to other programs via STDOUT to produce a video. This has only been tested with ffmpeg.
```
pxlslog-explorer.exe render -s logs/pixels_cXX.sanit.log --bg canvas/cXX.png --step 300000 | ffmpeg -f rawvideo -pixel_format rgba -video_size (width)x(height) -i pipe:0 ...
```

## The future
This program is certainly going to evolve as new use cases are discovered.
As such, the intention is to accept feedback and adapt to what users desire to suit their needs.
The scope of this program is intentially minimalistic so it can be expanded on or used as a foundation in other personal projects.

### Potential future features:
- Alter renders to produce Heatmaps or Virginmap
- An actual GUI
  - Probably not

### Potential improvements:
- General optimisations
- Alternative output formats (.csv, etc)
- Statistics generation
- More error handling (if you somehow mess up a log file)