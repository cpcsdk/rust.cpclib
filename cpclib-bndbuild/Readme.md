# Bndbuild

## Synopsis

Makefile-like tool tailored to build Amstrad CPC project.
It embeds various cpclib tools such as basm, m4, img2cpc but can still execute external programs such as rasm, winape.

The rules are described in a `yaml` file. Check for example the test project in <tests/dummy> folder.

## Help

```
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

  -V, --version
          Print version

  -f, --file <FILE>
          Provide the YAML file for the given project.

          [default: bndbuild.yml]

  -w, --watch
          Watch the targets and permanently rebuild them when needed.
```

## Example

Here is an example to build a dummy Amstrad CPC project and execute on the real machine thanks to the m4.
It is available in <tests/dummy>
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
- `basm` to launch [basm assembler](../cpclib-basm)
- `img2cpc` to make [image conversion](../cpclib-imgconverter)
- `rm` to delete files
- `xfer` to transfer to the CPC with the M4 card
- `extern` to launch any command available on the machine