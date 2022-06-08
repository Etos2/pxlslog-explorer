# PxlsLog-Explorer
A simple command line utility program to filter logs and generate timelapses for [pxls.space](https://pxls.space/).
Designed for pxls.space users to explore their personal logs or generate pretty timelapses.
Forgot where the logs are? They're right [here](https://pxls.space/x/logs/). Your personal hashes are located on your profile page.

### Current features:
- Simple program settings
  - Disable overwritting existing files
- Filter entries to file (Defaults to STDOUT)
  - Via provided date (Format: %Y-%m-%dT%H:%M:%S%.f)
  - Via colour index
  - Via region (Format: x1,y1,x2,y2)
  - Via actions (place, undo, overwrite, rollback, rollback-undo, nuke)
  - Via user hash
- Render logs into timelapses or individual frames
  - Customisable step (time passed between frames)
  - Supports multiples file formats via the [image](https://crates.io/crates/image) crate
  - Can pipe raw RGBA video data (e.g. ffmpeg)
  - Supports multiple styles (e.g. heat, virgin, activity, etc)
  - Can import custom palettes (.gpl, .aco, .csv, .txt (paint.NET)) including directly from [pxls](https://pxls.space/info)

## Help
To get on track, seek the help argument.
This will list basic arguments and display available subcommands.
Alternatively, use on a subcommand to display arguments for the subcommand.

```
pxlslog-explorer.exe --help
pxlslog-explorer.exe filter --help
pxlslog-explorer.exe render --help
```

## Filter
The filter subcommand is quite simple, follow the syntax hints for tricky filters such as "--after", "--before" and "--region".
Be wary with many filters applied, it can be quite messy:

```
// Print straight to STDOUT
pxlslog-explorer.exe filter pixels_cXX.sanit.log

// Write to mypixels_cXX.log when color == 5
pxlslog-explorer.exe filter --color 5 pixels_cXX.sanit.log mypixels_cXX.log

// Write to mypixels2_cXX.log when equal to hash and action = undo
pxlslog-explorer.exe filter --action undo --user (insert hash here) pixels_cXX.sanit.log mypixels2_cXX.log
```

## Render
The render subcommand accepts a log file and produces frames in the desired format. However it can also produce a single complete frame.
A palette can also be provided, either from raw json found [here](https://pxls.space/info) or palette files found [here](https://pxls.space/x/palette) [.gpl, .aco, .csv, .txt (paint.NET)].
See the [image](https://crates.io/crates/image) crate for supported image formats.

```
// Using background as source, produce a frame every 5 minutes in the PNG format
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --step 300000

// Or, produce a single frame
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot

// You can also provide a custom palette.
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot --palette p10.gpl
```

Additionally, frames can be piped to other programs via STDOUT to produce a video. This has only been tested with ffmpeg.
(Note that you need to specify the resolution)
```
pxlslog-explorer.exe render -s pixels_cXX.sanit.log --bg cXX.png --step 300000 | ffmpeg -f rawvideo -pixel_format rgba -video_size [...] -i pipe:0 ...
```

## The future
This program is certainly going to evolve as new use cases are discovered.
As such, the intention is to accept feedback and adapt to what users desire to suit their needs.
The scope of this program is intentially minimalistic so it can be expanded on or used as a foundation in other personal projects.

### Potential future features:
- Alternative output formats (.csv, etc)
- Statistics generation
- Ability to merge logs
- An actual GUI
  - Probably not