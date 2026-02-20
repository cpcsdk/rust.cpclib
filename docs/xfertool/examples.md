# XferTool Examples

!!! info "Command Reference"
    For current syntax: `cpclib-xfertool --help`
    Complete reference: [CLI Help Reference](../CLI_HELP_REFERENCE.md)

## Basic Usage

XferTool transfers files to/from Amstrad CPC via M4 WiFi.

### Upload File
```bash
cpclib-xfertool -p game.sna
```

### Upload and Run
```bash
cpclib-xfertool -y game.sna
```

### List Files
```bash
cpclib-xfertool --ls
```

### Reset CPC
```bash
cpclib-xfertool -s
```

For all options: `cpclib-xfertool --help`
