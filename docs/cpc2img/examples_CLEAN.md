# cpc2img Examples

!!! info "Complete Command Reference"
    For all current command syntax and options, see:
   
    - Built-in help: `cpc2img --help`
    - [Complete CLI Help Reference](../CLI_HELP_REFERENCE.md)

## Basic Usage

The general syntax is:
```bash  
cpc2img <INPUT> <OUTPUT> <SUBCOMMAND> [OPTIONS]
```

### Convert Screen Data
```bash
cpc2img screen.bin output.png screen
```

### Convert Sprite Data
```bash
cpc2img sprite.spr output.png sprite --width 16
```

### Extract Palette
```bash
cpc2img colors.pal palette.png palette
```

For complete options and advanced usage, use `--help` with each subcommand.
