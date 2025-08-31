# Bndbuild

## Synopsis

Crossdev tool tailored to build Amstrad CPC project although it can generalize to z80-related projects or even any buildable projects.
It embeds the Benediction crossdev ecosystem such as `basm`, `m4`, `img2cpc` but can still execute external programs such as `sjasmplus`, `rasm`, `winape`, `ace` it is able to download and install or any command chosen by the user.

It can be used  as a command launcher or a build system and is available as a command line and a graphical version.

As it is still in beta stage, I do not properly play with version numbering. This will be fixed as soon as there is a user base
using it.

## Command launcher

You can see `bndbuild` as a universal proxy to plenty of crossdev tools without manually installing them.
See the documentation and the `--direct` argument.
So if you are not a user of the other Benediction tools, and whatever you are using the build system, 
`bndbuild` can still ease your crossdev workflow by taking care of downloading, installing and launching tools.
See the help for the list of available tools. Fell free to request more in the issue tracker.

## Build system

You can see `bndbuild` as a build system similar to Makefile but with a different syntax and better integration.
The build rules are described in a `yaml` file templated by ` jinja`  engine. Check for example a simple test project at <https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/dummy> folder, or a more complicated one that use various commands and templating at <https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/ucpm>.



The documentation is quite minimal at the moment, but included examples code should be still valid and assembled properly. 
The user base being quite small, lots of bugs can remain. Do note hesitate to fill issues <https://github.com/cpcsdk/rust.cpclib/issues> or propose fixes.



## Installation

### Download

Prefer to compile yourself `bndbuild`. But you can still download latest versions here:

- [Command line version for Windows](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild.exe)
- [Command line version for Linux](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild)
- [Graphical version for Windows](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild-gui.exe)
- [Graphical version for Linux](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild-gui)
- [Installer for the experimental new graphical version for Windows](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild-tauri_0.1.0_x64-setup.exe)
- [Installer for the experimental new graphical version for Linux](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/bndbuild-tauri_0.1.0_amd64.AppImage)

Windows antivirus tend to flag `rust` programs as virus. Sadly it is the case for `bndbuild`.

### Compile

You need to install the `rust` toolchain with its nightly version to compile `bndbuild` (<https://rustup.rs/>) as well as some additional dependencies.

- unbuntu-like dependencies: libgtk-3-dev libcogl-pango-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev libxdo-dev
- windows: msvc
- macos: ??? it probably does not compile yet

```bash
$ git clone git@github.com:cpcsdk/rust.cpclib.git --depth 1
$ cd rust.cpclib/
$ cargo install --path cpclib-bndbuild          # For the command line version
$ cargo install --path cpclib-visual-bndbuild   # For the graphical version
$ cd cpclib-bndbuild-tauri && cargo tauri build # To create the new version. The folder target/release/bundle contains the installers
```

## Build format

The rules description file must respect the `yaml` text file format.
It can be named `bndbuild.yml`, `bnd.build` or `build.bnd` but this can be overridden by the `-f` argument.
It contains list of rules.
Each rule can have the following keys:

- `tgt`: to list the files build by the rule. Either all the files in one line or one file per line. 
- `dep`: to list the files needed to build the rule. Either all the files in one line or one file per line.
- `cmd`: a command, or a list of commands, executed by the rule. Commands prefixed by `-` can silently fail. `$@` is replaced by the first target and `$<` is replaced by the first dependency.>
- `help`: an optional help text to describe the rule.
- `phony`: an optional tag to express the rule does not generate anyfile (it is inferred when the commands are not extern). Mainly serves for the `--watch` argument.
- `constraint`: Allows to filter the rule for the specified expression
   
   * Functions: `hostname(MY_HOST)` is true if the machine is `MY_HOST`. `os(windows)`, `os(linux)`, and `os(macosx)` are true for the specified os.
   * Negation: `not(EXPRESSION)` is true when `EXPRESSION` is false
   * Combination: `and(EXPRESSION, EXPRESSION, ...)` and `or(EXPRESSION, EXPRESSION, ...)` allow to combine expressions


If you know how to configure your IDE to statically verify your yaml files, here is the configuration you can provide: <https://raw.githubusercontent.com/cpcsdk/rust.cpclib/refs/heads/master/cpclib-bndbuild/schema.json>

## Templating

A jinja-like templating is used to generate the final yaml file : <https://docs.rs/minijinja/latest/minijinja/syntax/index.html>.
So you can automatically generate rules with its macro system.

## Preset variables

- `FAP_INIT_PATH`: path to the assembled player initializer for fap
- `FAP_PLAY_PATH`: path to the assembled player for fap
- `AKG_PATH`: path the the AKG player


## Example

Here is an example to build a dummy Amstrad CPC project and execute on the real machine thanks to the m4.
It is available in [tests/dummy](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/dummy) (the repository does not contains the external tools needed to properly build the project. It is straightforward to add them).
Successive calls to the build task do nothing as soon as no file has been modified.
It is also possible to watch the dependencies of a given task to automatically build it when they are modified.
This cannot be seen with the capture, but each time m4 command is launched, the project is send into the CPC machine (it takes several seconds however).

![Animation](dummy.gif)


## Help

```
bndbuild --help`
Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any Benediction crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.

Benediction CPC demo project builder

Usage: bndbuilder [OPTIONS] [TARGET]...

Arguments:
  [TARGET]...
          Provide the target(s) to run.

Options:
  -h, --help [<CMD>]
          Show the help of the given subcommand CMD.
          
          [default: bndbuild]
          [possible values: cpc, emu, emuctrl, emucontrol, ace, acedl, winape, cpcec, amspirit, sugarbox, basm, assemble, orgams, rasm, sjasmplus, vasm, bndbuild, build, cp, copy, dsk, disc, echo, print, extern, fap, img2cpc, imgconverter, hideur, impdsk, impdisc, martine, rm, del, xfer, cpcwifi, m4]

      --direct
          Bypass the task file and directly execute a command along: [cpc, emu, emuctrl, emucontrol, ace, acedl, winape, cpcec, amspirit, sugarbox, basm, assemble, orgams, rasm, sjasmplus, vasm, bndbuild, build, cp, copy, dsk, disc, echo, print, extern, fap, img2cpc, imgconverter, hideur, impdsk, impdisc, martine, rm, del, xfer, cpcwifi, m4].

  -V, --version
          Print version

      --dot
          Generate the .dot representation of the selected bndbuild.yml file

      --show
          Show the file AFTER interpreting the templates

  -f, --file <FILE>
          Provide the YAML file for the given project.

  -w, --watch
          Watch the targets and permanently rebuild them when needed.

  -l, --list
          List the available targets

  -D, --define <DEFINE_SYMBOL>
          Provide a symbol with its value (default set to 1)

  -c, --clear-cache [<clear>]
          Clear cache folder that contains all automatically downloaded executables. Can optionally take one argument to clear the cache of the corresponding executable.
          
          [possible values: ace, acedl, winape, cpcec, amspirit, sugarbox, rasm, sjasmplus, vasm, fap, impdsk, impdisc, martine]

      --init
          Init a new project by creating it

  -a, --add <add>
          Add a new basm target in an existing bndbuild.yml (or create it)

  -d, --dep <dep>
          The source files

  -k, --kind <kind>
          The kind of command to be added in the yaml file
          
          [possible values: cpc, emu, emuctrl, emucontrol, ace, acedl, winape, cpcec, amspirit, sugarbox, basm, assemble, orgams, rasm, sjasmplus, vasm, bndbuild, build, cp, copy, dsk, disc, echo, print, extern, fap, img2cpc, imgconverter, hideur, impdsk, impdisc, martine, rm, del, xfer, cpcwifi, m4]

cpclib-bndbuild 0.6.0 embedded by cpclib-bndbuild 0.6.0
```

## Commands

The `--direct -- COMMAND [ARG...]` allows to directly launch a command without managing a build file. `COMMAND` can be any command accepted in `cmd` key (they are listed in the documentation of `--help`).
The commands are either included by the application (so limited to cpclib commands and os agnostic), or accessible externally (no limitation, but os dependent). Some command may have an alias.


Several commands need to be downloaded (so internet is required), assembled (so their prerequisites need to be installed).
There is no (yet) cleanup if download/compilation fail. So think to do `bndbuild --clear <cmd>` to cleanup manually.

### Display management: echo (echo)
```
Print the arguments.

Usage: echo [arguments]...

Arguments:
  [arguments]...
          Words to print
```

### External program management (extern)

```
Launch an external command.

Usage: extern <program> [arguments]...

Arguments:
  <program>
          The program to execute
  [arguments]...
          The arguments of the program
```




### Image management: Benediction transfer tool (im2cpc)

```
Simple CPC image conversion tool

Usage: CPC image conversion tool [OPTIONS] <SOURCE> [COMMAND]

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
  <SOURCE>
          Filename to convert

Options:
  -m, --mode <MODE>
          Screen mode of the image to convert.
          
          [default: 0]
          [possible values: 0, 1, 2]

      --fullscreen
          Specify a full screen displayed using 2 non consecutive banks.

      --overscan
          Specify an overscan screen (crtc meaning).

      --standard
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

      --pal <OCP_PAL>
          OCP PAL file. The first palette among 12 is used

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
          

  -V, --version
          Print version
```


### Image management: Impact transfer tool (martine)

```
Martine (0.39) [INFO] 2024/11/10 08:23:03 martine convert (jpeg, png format) image to Amstrad cpc screen (even overscan)
Martine (0.39) [INFO] 2024/11/10 08:23:03 By Impact Sid (Version:0.39)
  -address string
        Starting address to display sprite in delta packing (default "0xC000")
Martine (0.39) [INFO] 2024/11/10 08:23:03 Special thanks to @Ast (for his support), @Siko and @Tronic for ideas
Martine (0.39) [INFO] 2024/11/10 08:23:03 usage :
  -algo int

        Algorithm to resize the image (available : 
                1: NearestNeighbor (default)
                2: CatmullRom
                3: Lanczos
                4: Linear
                5: Box
                6: Hermite
                7: BSpline
                8: Hamming
                9: Hann
                10: Gaussian
                11: Blackman
                12: Bartlett
                13: Welch
                14: Cosine
                15: MitchellNetravali
                 (default 1)
  -analyzetilemap string
        analyse the image to get the most accurate tilemap according to the  criteria :
                size : lower export size
                number : lower number of tiles
  -animate
        Will produce an full screen with all sprite on the same image (add -in image.gif or -in *.png)
  -autoexec
        Execute on your remote CPC the screen file or basic file.
  -brightness float
        apply brightness on the color of the palette on amstrad plus screen. (max value 100 and only on CPC PLUS).
  -compiled
        Export sprite as compiled sprites.
  -contrast float
        apply contrast on the color of the palette on amstrad plus screen. (max value 100 and only on CPC PLUS).
  -delta
        Delta mode: compute delta between two files (prefixed by the argument -df)
                (ex: -delta -df file1.SCR -df file2.SCR -df file3.SCR).
                (ex with wildcard: -delta -df file\?.SCR or -delta file\*.SCR
  -deltapacking
        Will generate all the animation code from the followed gif file.
  -deltapacking2
        Will generate all the animation code from the followed gif file (and optimize export).
  -df value
        scr file path to add in delta mode comparison. (wildcard accepted such as ? or * file filename.) 
  -dithering int
        Dithering algorithm to apply on input image
        Algorithms available:
                0: FloydSteinberg
                1: JarvisJudiceNinke
                2: Stucki
                3: Atkinson
                4: Sierra
                5: SierraLite
                6: Sierra3
                7: Bayer2
                8: Bayer3
                9: Bayer4
                10: Bayer8
         (default -1)
  -dsk
        Copy files in a new CPC image Dsk.
  -egx1
        Create egx 1 output cpc image overscan (option -fullscreen) or classical (mix mode 0 / 1).
                (ex before generate two images one in mode 1 et one in mode 0
                for instance : martine -in myimage.jpg -mode 0 and martine -in myimage.jpg -mode 1
                : -egx1 -in 1.SCR -mode 0 -pal 1.PAL -in2 2.SCR -out test -mode2 1 -dsk)
                or
                (ex automatic egx from image file : -egx1 -in input.png -mode 0 -out test -dsk)
  -egx2
        Create egx 2 output cpc image overscan (option -fullscreen) or classical (mix mode 1 / 2).
                (ex before generate two images one in mode 1 et one in mode 2
                for instance : martine -in myimage.jpg -mode 0 and martine -in myimage.jpg -mode 1
                : -egx2 -in 1.SCR -mode 0 -pal 1.PAL -in2 2.SCR -out test -mode2 1 -dsk)
                or
                (ex automatic egx from image file : -egx2 -in input.png -mode 0 -out test -dsk)
  -extendeddsk
        Export in a Extended DSK 80 tracks, 10 sectors 400 ko per face
  -fillout
        Fill out the gif frames needed some case with deltapacking
  -flash
        generate flash animation with two ocp screens.
                (ex: -mode 1 -flash -in input.png -out test -dsk)
                or
                (ex: -mode 1 -flash -i input1.scr -pal input1.pal -mode2 0 -iin2 input2.scr -pal2 input2.pal -out test -dsk )
  -flat
        Export sprite as flat file.
  -fullscreen
        Overscan mode (default no overscan)
  -go
        Export results as .go1 and .go2 files.
  -height int
        Custom output height in pixels. (Will produce a sprite file .win) (default -1)
  -help
        Display help message
  -host string
        Set the ip of your M4.
  -imp
        Will generate sprites as IMP-Catcher format (Impdraw V2).
  -in string
        Picture path of the input file.
  -in2 string
        Picture path of the second input file (flash mode)
  -info
        Return the information of the file, associated with -pal and -win options
  -initprocess string
        Create a new empty process file.
  -ink string
        Path of the palette Cpc ink file. (Apply the input ink palette on the image)
  -inkswap string
        Swap ink:
                for instance mode 4 (4 inks) : 0=3,1=0,2=1,3=2
                will swap in output image index 0 by 3 and 1 by 0 and so on.
  -iter int
        Iterations number to walk in roll mode, or number of images to generate in rotation mode. (default -1)
  -iterx int
        Number of tiles on a row in the input image. (default 1)
  -itery int
        Number of tiles on a column in the input image. (default 1)
  -json
        Generate json format output.
  -keephigh int
        Bit rotation on the top and keep pixels (default -1)
  -keeplow int
        Bit rotation on the bottom and keep pixels (default -1)
  -kit string
        Path of the palette Cpc plus Kit file. (Apply the input kit palette on the image)
  -linewidth string
        Line width in hexadecimal to compute the screen address in delta mode. (default "#50")
  -losthigh int
        Bit rotation on the top and lost pixels (default -1)
  -lostlow int
        Bit rotation on the bottom and lost pixels (default -1)
  -mask string
        Mask to apply on each bit of the sprite (to apply an and operation on each pixel with the value #AA [in hexdecimal: #AA or 0xAA, in decimal: 170] ex: martine -in myimage.png -width 40 -height 80 -mask #AA -mode 0 -maskand)
  -maskand
        Will apply an AND operation on each byte with the mask
  -maskor
        Will apply an OR operation on each byte with the mask
  -mode int
        Output mode to use :
                0 for mode0
                1 for mode1
                2 for mode2
                and add -fullscreen option for overscan export.
                 (default -1)
  -mode2 int
        Output mode to use :
                0 for mode0
                1 for mode1
                2 for mode2
                mode of the second input file (flash mode) (default -1)
  -multiplier float
        Error dithering multiplier. (default 1.18)
  -noheader
        No amsdos header for all files (default amsdos header added).
  -ocpwin
        Export sprite as OCP win file.
  -oneline
        Display every other line.
  -onerow
        Display  every other row.
  -out string
        Output directory
TODO / handle string collect instead of stdout output
  -pal string

        Apply the input palette to the image
  -pal2 string
        Apply the input palette to the second image (flash mode)
  -plus
        Plus mode (means generate an image for CPC Plus Screen)
  -processfile string
        Process file path to apply.
  -quantization
        Use additionnal quantization for dithering.
  -reducer int
        Reducer mask will reduce original image colors. Available : 
                1 : lower
                2 : medium
                3 : strong
         (default -1)
  -remotepath string
        Remote path on your M4 where you want to copy your files.
  -reverse
        Transform .scr (overscan or not) file with palette (pal or kit file) into png file
  -rla int
        Bit rotation on the left and keep pixels (default -1)
  -roll
        Roll mode allow to walk and walk into the input file, associated with rla,rra,sra,sla, keephigh, keeplow, losthigh or lostlow options.
  -rotate
        Allow rotation on the input image, the input image must be a square (width equals height)
  -rotate3d
        Allow 3d rotation on the input image, the input image must be a square (width equals height)
  -rotate3dtype int
        Rotation type :
                1 rotate on X axis
                2 rotate on Y axis
                3 rotate reverse X axis
                4 rotate left to right on Y axis
                5 diagonal rotation on X axis
                6 diagonal rotation on Y axis
    
  -rotate3dx0 int
        X0 coordinate to apply in 3d rotation (default width of the image/2) (default -1)
  -rotate3dy0 int
        Y0 coordinate to apply in 3d rotation (default height of the image/2) (default -1)
  -rra int
        Bit rotation on the right and keep pixels (default -1)
  -scanlinesequence string
        Scanline sequence to apply on sprite. for instance : 
                martine -in myimage.jpg -width 4 -height 4 -scanlinesequence 0,2,1,3 
                will generate a sprite stored with lines order 0 2 1 and 3.
    
  -sla int
        Bit rotation on the left and lost pixels (default -1)
  -sna
        Copy files in a new CPC image Sna.
```

### File management: cp (cp,copy)

```
Copy files.


Usage: cp [arguments]...

Arguments:
  [arguments]...
          Files to copy. Last one being the destination

Inner command of cpclib-bndbuild 0.6.0
```

### File management: rm (rm, del)

```
Delete files.


Usage: rm [arguments]...

Arguments:
  [arguments]...
          Files to delete.

Inner command of cpclib-bndbuild 0.6.0
```

### Amsdos header management (hideur)

```
Usage: hideur [OPTIONS] <INPUT>

Arguments:
  <INPUT>
          Input file to manipulate

Options:
      --info
          

  -o, --output <OUTPUT>
          Output file to generate

  -u, --user <USER>
          User where to put the file

  -t, --type <TYPE>
          File type
          
          [possible values: 0, 1, 2, Basic, Protected, Binary, basic, protected, binary, BASIC, PROTECTED, BINARY]

  -x, --execution <EXEC>
          Execution address. Default to the load address if not specified.

  -l, --load <LOAD>
          Loading address.
```

### Disc management: Benediction dsk manager (dsk,disc)

```
Manipulate DSK files

Usage: dsk_manager <DSK_FILE> [COMMAND]

Commands:
  format   Format a dsk
  catalog  Manipulate the catalog. Can only works for DSK having a Track 0 compatible with Amsdos
  get      Retrieve files for the disc in the Amsdos way
  add      Add files in the disc in an Amsdos way
  put      Add files in the disc in a sectorial way
  help     Print this message or the help of the given subcommand(s)

Arguments:
  <DSK_FILE>
          DSK file to manipulate

Options:
  -h, --help
          

  -V, --version
          Print version

cpclib-disc 0.8.2 embedded by cpclib-bndbuild 0.6.0
```


### Disc management: Impact dsk manager (impdsk)

```
Here sample usages :
        * Create empty simple dsk file : dsk -dsk output.dsk -format
        * Create empty simple dsk file with custom tracks and sectors: dsk -dsk output.dsk -format -sector 8 -track 42
        * Create empty extended dsk file with custom head, tracks and sectors: dsk -dsk output.dsk -format -sector 8 -track 42 -dsktype 1 -head 2
        * Create empty sna file : dsk -sna output.sna
        * List dsk content : dsk -dsk output.dsk -list
        * Get information on Sna file : dsk -sna output.sna -info
        * Get information on file in dsk  : dsk -dsk output.dsk -amsdosfile hello.bin -info
        * List file content in hexadecimal in dsk file : dsk -dsk output.dsk -amsdosfile hello.bin -hex
        * Put file in dsk file : dsk -dsk output.dsk -put -amsdosfile hello.bin -exec #1000 -load 500
        * Put file in sna file (here for a cpc plus): dsk -sna output.sna -put -amsdosfile hello.bin -exec #1000 -load 500 -screenmode 0 -cpctype 4


  -addheader
        Add header to the standalone file (must be set with exec, load and type options).
  -amsdosfile string
        File to handle in (or to insert in) the dsk.
  -analyze
        Returns the DSK header
  -ascii
        list the amsdosfile in ascii mode.
  -autoextract string
        Extract all DSK contained in the folder path
  -autotest
        Executs all tests.
  -basic
        List a basic amsdosfile.
  -cpctype int
        CPC type (sna import feature): 
                CPC464 : 0
                CPC664: 1
                CPC6128 : 2
                Unknown : 3
                CPCPlus6128 : 4
                CPCPlus464 : 5
                GX4000 : 6
                 (default 2)
  -data
        Format in vendor format (sectors number #09, end track #27) (default true)
  -desassemble
        list the amsdosfile desassembled.
  -dsk string
        Dsk path to handle.
  -dsktype int
        DSK Type :
                0 : DSK
                1 : EDSK
                3 : SNA
    
  -exec string
        Execute address of the inserted file. (hexadecimal #170 allowed.)
  -force
        Force overwriting of the inserted file.
  -format
        Format the followed dsk or sna.
  -get
        Get the file in the dsk.
  -head int
        Number of heads in the DSK (format) (default 1)
  -help
        display extended help.
  -hex
        List the amsdosfile in hexadecimal.
  -info
        Get informations of the amsdosfile (size, execute and loading address). Or get sna informations.
  -list
        List content of dsk.
  -load string
        Loading address of the inserted file. (hexadecimal #170 allowed.)
  -put
        Put the amsdosfile in the current dsk.
  -quiet
        remove useless display (for scripting for instance)
  -rawexport
        raw exports the amsdosfile, this option is associated with -dsk, -track and -sector.
        This option will do a raw extract of the content beginning to track and sector values and will stop when size is reached.
        for instance : dsk -dsk mydskfile.dsk -amsdosfile file.bin -rawexport -track 1 -sector 0 -size 16384
  -rawimport
        raw imports the amsdosfile, this option is associated with -dsk, -track and -sector.
        This option will do a raw copy of the file starting to track and sector values.
        for instance : dsk -dsk mydskfile.dsk -amsdosfile file.bin -rawimport -track 1 -sector 0
  -remove
        Remove the amsdosfile from the current dsk.
  -screenmode int
        screen mode parameter for the sna. (default 1)
  -sector int
        Sector number (format). (default 9)
  -size int
        Size to extract in rawexport, see rawexport for more details.
  -sna string
        SNA file to handle
  -snaversion int
        Set the sna version (1 or 2 available). (default 1)
  -track int
        Track number (format). (default 39)
  -type string
        Type of the inserted file 
                ascii : type ascii
                protected : type ascii protected
                binary : type binary
    
  -user int
        User number of the inserted file.
  -vendor
        Format in vendor format (sectors number #09, end track #27)
  -version
        Display the app's version and quit.
```


### Assembler: BASM (basm)

```
Benediction ASM -- z80 assembler that mainly targets Amstrad CPC

Usage: basm [OPTIONS] [INPUT]

Arguments:
  [INPUT]
          Input file to read.

Options:
      --inline <INLINE>
          Z80 code is provided inline

  -o, --output <OUTPUT>
          Filename of the output.

      --db
          Write a db list on screen (usefull to get the value of an opcode)

      --lst <LISTING_OUTPUT>
          Filename of the listing output.

      --remu <REMU_OUTPUT>
          Filename to store the remu file used by Ace to import label and debug information

      --wabp <WABP_OUTPUT>
          Filename to stare the WABP file use to provide Winape breakpoints

      --breakpoint-as-opcode
          Breakpoints are stored as opcodes (mainly interesting for winape emulation)

      --sym <SYMBOLS_OUTPUT>
          Filename of the output symbols file.

      --sym_kind <SYMBOLS_KIND>
          Format of the output symbols file
          
          [default: basm]
          [possible values: winape, basm]

      --basic
          Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive).

      --binary
          Request a binary header

      --cartridge
          Generate a CPR

      --snapshot
          Generate a snapshot

      --nochunk <CODE>
          Deactivate some snapshot chunks (comma separated)
          
          [possible values: BRKC, BRKS, REMU, SYMB]

  -i, --case-insensitive
          Configure the assembler to be case insensitive.

      --disable-warnings
          Do not generate warnings

  -d, --directives-prefixed-by-dot
          Expect directives to by prefixed with a dot

  -I, --include <INCLUDE_DIRECTORIES>
          Provide additional directories used to search files

  -D, --define <DEFINE_SYMBOL>
          Provide a symbol with its value (default set to 1)

      --no-forced-void
          By default (void) is mandatory on macro without parameters. This option disable this behavior

      --debug
          Trace more information to help debug

      --override
          Override file when already stored in a disc

      --backup
          Backup an existing file when saved on disc

      --orgams
          Main source is at ORGAMS format

      --m4 <TO_M4>
          Provide the IP address of the M4

  -l <LOAD_SYMBOLS>
          Load symbols from the given file

      --Werror
          Warning are considered to be errors

      --progress
          Show a progress bar.

      --list-embedded
          List the embedded files

      --view-embedded <VIEW_EMBEDDED>
          Display one specific embedded file
          
          [possible values: inner://crtc.asm, inner://deexo.asm, inner://deshrink.asm, inner://dzx0_fast.asm, inner://dzx0_standard.asm, inner://firmware/amsdos.asm, inner://firmware/casmng.asm, inner://firmware/gfxvdu.asm, inner://firmware/highkern.asm, inner://firmware/indirect.asm, inner://firmware/kernel.asm, inner://firmware/keymng.asm, inner://firmware/lowkern.asm, inner://firmware/machine.asm, inner://firmware/math6128.asm, inner://firmware/mathnot464.asm, inner://firmware/mathnot6xx.asm, inner://firmware/not464.asm, inner://firmware/scrpack.asm, inner://firmware/sound.asm, inner://firmware/txtvdu.asm, inner://ga.asm, inner://lz48decrunch.asm, inner://lz49decrunch.asm, inner://lz4_docent.asm, inner://opcodes_first_byte.asm, inner://pixels-routs.asm, inner://unaplib.asm, inner://unaplib_fast.asm]

  -h, --help
          

  -V, --version
          Print version

cpclib-basm 0.8.3 embedded by cpclib-bndbuild 0.6.0
```

### Assembler: RASM (rasm)

```
RASM v2.2.9 (build xx/10/2024) - Consolidation
(c) 2017 Edouard BERGE (use -n option to display all licenses / -autotest for self-testing)
LZ4 (c) Yann Collet / ZX0 & ZX7 (c) Einar Saukas / Exomizer 2 (c) Magnus Lind / LZSA & AP-Ultra (c) Emmanuel Marty

SYNTAX: rasm <inputfile> [options]

FILENAMES:
-oa              automatic radix from input filename
-o  <radix>      choose a common radix for all files
-or <filename>   choose a radix filename for ROM output
-ob <filename>   choose a full filename for binary output
-oc <filename>   choose a full filename for cartridge output
-ol <filename>   choose a full filename for ROM label output
-oi <filename>   choose a full filename for snapshot output
-os <filename>   choose a full filename for symbol output
-ot <filename>   choose a full filename for tape output
-ok <filename>   choose a full filename for breakpoint output
-I<path>         set a path for files to read
-no              disable all file output
DEPENDENCIES EXPORT:
-depend=make     output dependencies on a single line
-depend=list     output dependencies as a list
if 'binary filename' is set then it will be outputed first
SYMBOLS EXPORT:
-rasm            export super symbols file for ACE-DL
-s               export symbols %s #%X B%d (label,adr,cprbank)
-sz              export symbols with ZX emulator convention
-sp              export symbols with Pasmo convention
-sw              export symbols with Winape convention
-ss              export symbols in the snapshot (SYMB chunk for ACE)
-sc <format>     export symbols with source code convention
-sm              export symbol in multiple files (one per bank)
-ec              export labels with original case
-er              export ROM labels
-l  <labelfile>  import symbol file (winape,pasmo,rasm)
-eb              export breakpoints
-wu              warn for unused symbols (alias, var or label)
SYMBOLS ADDITIONAL OPTIONS:
-sl              export also local symbol
-sv              export also variables symbol
-sq              export also EQU symbol
-sa              export all symbols (like -sl -sv -sq option)
-Dvariable=value import value for variable
COMPATIBILITY:
-m               Maxam style calculations
-dams            Dams 'dot' label convention
-ass             AS80  behaviour mimic
-uz              UZ80  behaviour mimic
-pasmo           PASMO behaviour mimic
-amper           use ampersand for hex values
-msep            <separator> set separator for modules
-utf8            convert symbols from french or spanish keyboard inside quotes
-fq              do not bother with special chars inside quotes
MISCELLANEOUS:
-quick           enable fast mode for ZX0 crunching
-cprquiet        do not display ROM detailed informations
-map             display information during early assembling stages
EDSK generation/update:
-eo              overwrite files on disk if it already exists
SNAPSHOT:
-sb              export breakpoints in snapshot (BRKS & BRKC chunks)
-ss              export symbols in the snapshot (SYMB chunk for ACE)
-v2              export snapshot version 2 instead of version 3
PARSING:
-me <value>      set maximum number of error (0 means no limit)
-twe             treat warnings as errors
-xr              extended error display
-w               disable warnings
-void            force void usage with macro without parameter
-mml             allow macro usage with parameters on multiple lines
```


### Assembler: Orgams (orgams)

Orgams is a native assembler. So, an emulator is used to assemble source.
WARNING: it currently does not work properly under windows.

```
Usage: orgams [OPTIONS] --from <DATA_SOURCE> --src <SRC>

Options:
  -f, --from <DATA_SOURCE>
          Data source (a folder for using albireo or a disc image)

  -s, --src <SRC>
          Filename to assemble or edit

  -d, --dst <DST>
          Filename to save after assembling. By default use the one provided by orgams

  -b, --basm2orgams
          Convert a Z80 source file into an ascii orgams file

  -e, --edit
          Launch the editor in an emulator

  -j, --jump
          Jump on the program instead of saving it

```

### Assembler: Sjasmplus (sjamsplus)

```
SjASMPlus Z80 Cross-Assembler v1.20.3 (https://github.com/z00m128/sjasmplus)
Based on code of SjASM by Sjoerd Mastijn (http://www.xl2s.tk)
Copyright 2004-2023 by Aprisobal and all other participants

Usage:
sjasmplus [options] sourcefile(s)

Option flags as follows:
  -h or --help[=warnings]  Help information (you see it)
  --zxnext[=cspect]        Enable ZX Spectrum Next Z80 extensions (Z80N)
  --i8080                  Limit valid instructions to i8080 only (+ no fakes)
  --lr35902                Sharp LR35902 CPU instructions mode (+ no fakes)
  --outprefix=<path>       Prefix for save/output/.. filenames in directives
  -i<path> or -I<path> or --inc=<path> ( --inc without "=" to empty the list)
                           Include path (later defined have higher priority)
  --lst[=<filename>]       Save listing to <filename> (<source>.lst is default)
  --lstlab[=sort]          Append [sorted] symbol table to listing
  --sym=<filename>         Save symbol table to <filename>
  --exp=<filename>         Save exports to <filename> (see EXPORT pseudo-op)
  --raw=<filename>         Machine code saved also to <filename> (- is STDOUT)
  --sld[=<filename>]       Save Source Level Debugging data to <filename>
 Note: use OUTPUT, LUA/ENDLUA and other pseudo-ops to control output
 Logging:
  --nologo                 Do not show startup message
  --msg=[all|war|err|none|lst|lstlab]
                           Stderr messages verbosity ("all" is default)
  --fullpath               Show full path to file in errors
  --color=[on|off|auto]    Enable or disable ANSI coloring of warnings/errors
 Other:
  -D<NAME>[=<value>] or --define <NAME>[=<value>]
                           Define <NAME> as <value>
  -                        Reads STDIN as source (even in between regular files)
  --longptr                No device: program counter $ can go beyond 0x10000
  --reversepop             Enable reverse POP order (as in base SjASM version)
  --dirbol                 Enable directives from the beginning of line
  --dos866                 Encode from Windows codepage to DOS 866 (Cyrillic)
  --syntax=<...>           Adjust parsing syntax, check docs for details.
Failure
Error while launching the command.
```


### Assembler: Vasm z80 oldstyle (vasm)

<http://sun.hasenbraten.de/vasm/release/vasm_6.html>


### Emulator-agnostic emulation (cpc,emu)

```
bndbuild --help cpc`
Usage: cpclib-runner [OPTIONS] <COMMAND>

Commands:
  orgams  
  run     
  help    Print this message or the help of the given subcommand(s)

Options:
  -a, --drivea <DISCA>
          Disc A image

  -b, --driveb <DISCB>
          Disc B image

      --albireo <FOLDER>
          Albireo content (only for ACE) - WARNING. It is destructive as it completely replaces the existing content

      --snapshot <SNAPSHOT>
          Specify the snapshot to launch

  -m, --memory <MEMORY>
          [possible values: 64, 128, 192, 256, 320, 576, 1088, 2112]

  -e, --emulator <EMULATOR>
          [default: ace]
          [possible values: ace, winape, cpcec, amspirit, sugarbox]

  -k, --keepemulator
          Keep the emulator open after the interaction

  -c, --clear-cache
          Clear the cache folder

  -d, --debug <DEBUG>
          rasm-compatible debug file (for ace ATM)

  -r, --auto-run-file <AUTO_RUN_FILE>
          The file to run

      --disable-rom <DISABLE_ROM>
          List the ROMS to deactivate
          
          [possible values: orgams, unidos]
```

### Emulator: AMSpiriT (amspirit)

```
MSpiriT peut être exécuté par une ligne de commande, en mode console par exemple, permettant
d’automatiser certaines séquences de démarrage.
De nouvelles commandes seront progressivement ajoutées selon les besoins.
Commandes disponibles :
Les commandes en ligne sont standardisées.
--autorun Exécute automatiquement un enregistrement Cassette
--crtc=X Fixe le type de CRTC au démarrage (X = 0, 1, 1b, 2 ou 4)
--file=file Charge un fichier dsk, ipf, hfe, cdt, wav, sna (le chemin doit être complet)
--csl=file Charge un fichier script « Cpc Scripting Language » (le chemin doit être complet)
--fullscreen Exécute AmspiriT en mode plein écran
--joystick Active le joystick (Mapping clavier)
--keybPC Clavier en mode mapping PC => CPC
--keybCPC Clavier en mode CPC (pas de mapping) – Disponible sur quelques claviers
--nojoystick Désactive le josystick
--mute Désactive le son
--romX=file_rom Charge un fichier ROM dans un emplacement X (X varie entre 1 et 15)
A noter que les ROMs chargées ne seront pas mémorisées par AmspiriT
--run=Filename Lance un programme présent sur une disquette ou une Rom.
--config-file=rep Fixe le répertoire de AmspiriT où se situe le fichier de configuration
```


### Emulator: CPCEC (cpcec)

 (link broken ATM)

### Emulator: SugarboxV2 (sugarbox)

(pthread/glibc issue on linux ATM)

### Emulator: Winape (winape)

```
 When starting WinAPE a disc image filename can be specified as a parameter (without the slash option). The following parameters can be specified on the command line:

Parameter       Function
filename        Specify the filename for the disc image to be used in Drive A:
/A      Automatically run the program in Drive A:. To specify the name of the program to run use /A:filename. To start a disc using a CP/M boot sector use /A:|CPM
/T:filename     Automatically start typing from the given Auto-type file.
/SN:filename    Specify a Snapshot file to be loaded and automatically started.
/SYM:filename   Load a file containing assembler/debugger symbols.
/SHUTDOWN       Shut down Windows when WinAPE is closed. Use /SHUTDOWN:FORCE to force shutdown if required.

For example, to start WinAPE using the disc image frogger.dsk contained within a Zip file frogger.zip and run the program named frogger use:

WinAPE frogger.zip\:frogger.dsk /a:frogger            
```


### Transfer: M4 support (xfer)

```
RUST version of the communication tool between a PC and a CPC through the CPC Wifi card

Usage: CPC xfer to M4 [CPCADDR] [COMMAND]

Commands:
  -r     Reboot M4.
  -s     Reboot CPC.
  -p     Upload the given file in the current folder or the provided one
  -y     Upload a file on the M4 in the /tmp folder and launch it. V3 snapshots are automatically downgraded to V2 version
  -x     Execute a file on the cpc (executable or snapshot)
  --ls   Display contents of the M4
  --pwd  Display the current working directory selected on the M4
  --cd   Change of current directory in the M4.
  help   Print this message or the help of the given subcommand(s)

Arguments:
  [CPCADDR]
          Specify the address of the M4. This argument is optional. If not set up, the content of the environment variable CPCIP is used.

Options:
  -V, --version
          Print version

  -h, --help
          

cpclib-xfertool 0.8.1 embedded by cpclib-bndbuild 0.6.0
```