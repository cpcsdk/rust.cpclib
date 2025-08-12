```
img2cpc.exe --help`
Profile debug compiled: Tue, 15 Aug 2023 12:27:00 +0000

Simple CPC image conversion tool

Usage: img2cpc.exe [OPTIONS] <SOURCE> [COMMAND]

Commands:
  sna     Generate a snapshot with the converted image.
  dsk     Generate a DSK with an executable of the converted image.
  scr     Generate an OCP SCR file
  exec    Generate a binary file to manually copy in a DSK or M4 folder.
  sprite  Generate a sprite file to be included inside an application
  tile    Generate a list of sprites
  m4      Directly send the code on the M4 through a snapshot
  help    Print this message or the help of the given subcommand(s)

Arguments:
  <SOURCE>  Filename to convert

Options:
  -m, --mode <MODE>
          Screen mode of the image to convert. [default: 0] [possible values: 0, 1, 2]
      --fullscreen <FULLSCREEN>
          Specify a full screen displayed using 2 non consecutive banks.
      --overscan <OVERSCAN>
          Specify an overscan screen (crtc meaning).
      --standard <STANDARD>
          Specify a standard screen manipulation.
  -s, --skipoddpixels
          Skip odd pixels when reading the image (usefull when the picture is mode 0 with duplicated pixels 
      --columnstart <PIXEL_COLUMN_START>
          Number of pixel columns to skip on the left.
      --columnskept <PIXEL_COLUMNS_KEPT>
          Number of pixel columns to keep.
      --linestart <PIXEL_LINE_START>
          Number of pixel lines to skip.
      --lineskept <PIXEL_LINES_KEPT>
          Number of pixel lines to keep.
      --pens <PENS>
          Separated list of ink number. Use ',' as a separater
      --pen0 <PEN0>
          Ink number of the pen 0
      --pen1 <PEN1>
          Ink number of the pen 1
      --pen2 <PEN2>
          Ink number of the pen 2
      --pen3 <PEN3>
          Ink number of the pen 3
      --pen4 <PEN4>
          Ink number of the pen 4
      --pen5 <PEN5>
          Ink number of the pen 5
      --pen6 <PEN6>
          Ink number of the pen 6
      --pen7 <PEN7>
          Ink number of the pen 7
      --pen8 <PEN8>
          Ink number of the pen 8
      --pen9 <PEN9>
          Ink number of the pen 9
      --pen10 <PEN10>
          Ink number of the pen 10
      --pen11 <PEN11>
          Ink number of the pen 11
      --pen12 <PEN12>
          Ink number of the pen 12
      --pen13 <PEN13>
          Ink number of the pen 13
      --pen14 <PEN14>
          Ink number of the pen 14
      --pen15 <PEN15>
          Ink number of the pen 15
  -h, --help
          Print help
  -V, --version
          Print version
```