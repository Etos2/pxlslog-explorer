# PxlsLog-Explorer
A simply command line utility program to filter logs and generate timelapses for [pxls.space](https://pxls.space/).
Designed for pxls.space users to explore their personal logs or to be utilised in mass analysis.
Forgot where the logs are? They're [here](https://pxls.space/x/logs/).

### Current features:
- Simple program settings
  - Disable overwritting existing files
- Filter entries to file (Defaults to STDOUT, be warned)
  - Via provided date (Format: %Y-%m-%dT%H:%M:%S%.f)
  - Via colour index
  - Via region (Format: x1,y1,x2,y2)
  - Via actions (place, undo, overwrite, rollback, rollback-undo, nuke)
  - Via user hash

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