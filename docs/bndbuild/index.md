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


## Real-world Projects

Here are some demo projects that use bndbuild as their build system:

- [**Blight**](https://github.com/rgiot/demo.bnd5.blight) - Demo released at Benediction party 5
- [**4deKades**](https://github.com/rgiot/demo.revision2025.4deKades) - Demo presented at Revision 2025
- [**Etchy**](https://github.com/rgiot/demo.revision2024.etchy) - Demo presented at Revision 2024

These projects demonstrate real-world usage of bndbuild's features including templating, multi-assembler support, graphics conversion, and automated build pipelines.


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

Bndbuild integrates many tools to support the complete CPC development workflow. These can be invoked directly using `--direct -- COMMAND [ARG...]` without needing a build file.

For a complete reference of all available commands and their options, see [Commands Reference](commands.md).

### Available Command Categories

- **Display & External**: `echo`, `extern`
- **Image Conversion**: 
  - [img2cpc](../img2cpc/) (im2cpc) - Benediction image converter
  - `martine` - Impact image converter
- **File Management**: `cp`, `rm`, `hideur`
- **Disc Management**: 
  - `dsk`, `disc` - Benediction DSK manager (embedded-only)
  - [catalog](../catalog/) (cat) - Catalog listing tool
  - `impdsk` - Impact DSK manager  
- **Assemblers**: 
  - [BASM](../basm/) - Benediction assembler
  - `rasm`, [orgams/borgams](../borgams/), `sjasmplus`, `vasm`
- **Emulators**: 
  - `cpc/emu` - Emulator-agnostic interface
  - `ace`, `winape`, `cpcec`, `amspirit`, `sugarbox`
- **Transfer**: [xfertool](../xfertool/) (xfer) - M4 support

Run `bndbuild --help <command>` to see detailed help for any specific command.

