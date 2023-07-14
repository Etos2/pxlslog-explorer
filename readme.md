# PxlsLog-Explorer
A simple command line utility program to filter logs and generate timelapses for [pxls.space](https://pxls.space/).
Designed for pxls.space users to explore their personal logs or generate pretty timelapses.
Forgot where the logs are? They're right [here](https://pxls.space/x/logs/). Your personal hashes are located on your profile page.

### Current features:
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
  - Render with specific size
- Simple program settings
  - Disable overwritting existing files
  - Quit immediately on soft errors
  - Easily simulate a command and observe results

## Basic usage
To get on track, use the help argument.
This will list basic arguments and display available subcommands.
Alternatively, use on a subcommand to display arguments for the subcommand.

```
pxlslog-explorer.exe --help
pxlslog-explorer.exe filter --help
pxlslog-explorer.exe render --help
```

The basic command layout is as follows:
```
pxlslog-explorer.exe --src [OPTIONS] --src <PATH> [COMMAND] <args>
pxlslog-explorer.exe --src [OPTIONS] --src <PATH> filter [OPTIONS] [COMMAND] <args>
```

## Filter
The filter subcommand is an **optional** command to remove undesired actions with multiple filters.
Each filter can accept multiple arguments, follow the syntax hints for tricky filters such as "--after", "--before" and "--region".
Be wary when using many filters, it can be quite messy:

Arg | Arg type | Result
--- | --- | ---
--after | TIMESTAMP | Only include actions after this date
--before | TIMESTAMP | Only include actions before this date
--color | INT | Only include actions with this color
--region | INT INT INT INT | Only include actions within this area
--user | STRING | Only include actions from these users
--user-src | PATH | Only include actions from these users, specified in a file seperated by newlines
--action | ENUM | Only include actions that are of a particular type [place, undo, overwrite, rollback, rollback-undo, nuke]

Examples:
```
// Write to file when color == 5
pxlslog-explorer.exe --src pixels_cXX.sanit.log filter --color 5 --dst mypixels_cXX.log

// Write to file action belongs to user and is of type UNDO
pxlslog-explorer.exe --src pixels_cXX.sanit.log filter --action undo --user Etos2 --dst mypixels_cXX.log
```

## Render
The render subcommand produces frames in the desired format. This can be used to generate full timelapses or a single image of the final canvas.
A palette can also be provided, either from raw json found [here](https://pxls.space/info) or palette files found [here](https://pxls.space/x/palette) [.gpl, .aco, .csv, .txt (paint.NET)].
See the [image](https://crates.io/crates/image) crate for supported image formats.

The following styles are supported:
- Normal:       Simulate pxls canvas
- Heat:         Simulate pxls heat map 
- Virgin:       Simulate pxls virgin map 
- Activity:     Generate a heat map indicating most active pixels
- Action:       Map pixel type to color (Magenta = Undo, Blue = Place, Cyan = Mod Overwrite, Green = Rollback, Yellow = Rollback undo, Red = Nuke)
- Milliseconds: Map pixel placement time within a millisecond to a color
- Seconds:      Map pixel placement time within a second to a color
- Minutes:      Map pixel placement time within a minute to a color
- Combined:     Above methods combined into a single render
- Age:          Generate a brightness map, where darker pixels are older

Examples:
```
// Using background as source, produce a frame every 5 minutes in the PNG format
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --step 5m normal

// Or, produce a single frame of the final state
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot normal

// Or, use a different style
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot virgin

// You can also provide a custom palette.
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot --palette p10.gpl normal

// Additionally, define the area of the render
pxlslog-explorer.exe render -s pixels_cXX.sanit.log -d cXX.png --bg cXX.png --screenshot --region x y width height normal
```

Additionally, frames can be piped to other programs via STDOUT to produce a video. This has only been tested with ffmpeg so far.
Note that you need to specify the resolution in this example.
```
pxlslog-explorer.exe render -s pixels_cXX.sanit.log --bg cXX.png --step 5m  normal | ffmpeg -f rawvideo -pixel_format rgba -video_size <WIDTH>x<HEIGHT> -i pipe:0 ...
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
