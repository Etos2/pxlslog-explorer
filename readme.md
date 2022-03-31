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
This will list basic arguments and display available subcommands:

```
pxlslog-explorer.exe --help
```

Currently, the filter subcommand is the only implemented subcommand.
It too has a help argument which explains how to use it:

```
pxlslog-explorer.exe filter --help
```

Inputs to this command are quite simple, follow the syntax hints for tricky filters such as "--after", "--before" and "--region".
Be wary with many filters applied, it can be quite messy:

```
// Print straight to STDOUT
pxlslog-explorer.exe filter pixels_cXX.sanit.log

// Write to mypixels_cXX.log when color == 5
pxlslog-explorer.exe filter --color 5 pixels_cXX.sanit.log mypixels_cXX.log

// Write to mypixels2_cXX.log when color == 5, after == "2021-04-12T23:56:04" and user = "insert hash here"
pxlslog-explorer.exe filter --color 5 --after 2021-04-12T23:56:04 --user (insert hash here) pixels_cXX.sanit.log mypixels2_cXX.log
```


## The future
This program is certainly going to evolve as new use cases are discovered.
As such, the intention is to accept feedback and adapt to what users desire to suit their needs.
The scope of this program is intentially minimalistic so it can be expanded on or used as a foundation in other personal projects.
Implementing rendering capabilities is already a stretch, and this should be considered when viewing the potential additions below.

### Potential future features:
- Render subcommand
  - Generate timelapses
  - Generate images
  - Alter renders to produce Heatmaps or Virginmaps
  - Consequently integrating palettes and canvas backgrounds
- An actual GUI
  - Probably not

### Potential improvements:
- General optimisations
- More friendly input methods
  - Accept a file for user hash
  - Accept multiple inputs per filter
- Alternative output formats (.csv, etc)
- Statistics generation
- More error handling (if you somehow mess up a log file)
- Verbosity setting (if needed)
