# Bndbuild

## Synopsis

Makefile-like tool tailored to build Amstrad CPC project.
It embeds the Benediction crossdev ecosystem such as `basm`, `m4`, `img2cpc` but can still execute external programs such as `rasm`, `winape`, `ace` it is able to download and install or any command.

The rules are described in a `yaml` file. Check for example a simple test project at <https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/dummy> folder, or a more complicated one that use various commands and templating at <https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/ucpm>.

## Help

```
bndbuild --help`
Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any benedicition crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.

Benediction CPC demo project builder

Usage: bndbuilder [OPTIONS] [TARGET]...

Arguments:
  [TARGET]...
          Provide the target(s) to run.

Options:
  -h, --help [<CMD>]
          Show the help of the given subcommand CMD.
          
          [default: bndbuild]
          [possible values: img2cpc, basm, rm, bndbuild, xfer]

      --direct
          Directly execute a command without trying to read a task file

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

      --init
          Init a new project by creating it

  -a, --add <add>
          Add a new basm target in an existing bndbuild.yml (or create it)

  -d, --dep <dep>
          The source files

  -k, --kind <kind>
          The kind of command to be added in the yaml file
          
          [possible values: basm, img2cpc, xfer]
```

## Example

Here is an example to build a dummy Amstrad CPC project and execute on the real machine thanks to the m4.
It is available in [tests/dummy](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/dummy) (the repository does not contains the external tools needed to properly build the project. It is straighforward to add them).
Successive calls to the build task do nothing as soon as no file has been modified.
It is also possible to watch the dependencies of a given task to automatically build it when they are modified.
This cannot be seen with the capture, but each time m4 command is launched, the project is send into the CPC machine (it takes several seconds however).

![Animation](dummy.gif)

## Format

The rules description file must respect the `yaml` text file format.
It is preferably named `bndbuild.yml` but this can be overriden by the `-f` argument.
It contains list of rules.
Each rule can have the following keys:

- `tgt`: to list the files build by the rule
- `dep`: to list the files needed to build the rule
- `cmd`: a command, or a list of commands, executed by the rule. Commands prefixed by `-` can silently fail
- `help`: an optional help text to describe the rule
- `phony`: an optional tag to express the rule does not generate anyfile (it is infered when the commands are not extern). Mainly serve for the `--watch` argument.

The commands are either included by the application (so limited to cpclib commands and os agnostic) or accessible externally (no limitation, but os dependent).
They are:
- `basm` to launch [basm assembler](../BASM)
- `bndbuild` to launch ` bndbuild` itself for another task or configuration file
- `img2cpc` to make [image conversion](../cpclib-imgconverter)
- `rm` to delete files
- `xfer` to transfer to the CPC with the M4 card
- `extern` to launch any command available on the machine

## Templating

A jinja-like templating is used to generate the final yaml file : <https://docs.rs/minijinja/latest/minijinja/syntax/index.html>.