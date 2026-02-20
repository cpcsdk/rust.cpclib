# CPC Runner Command Line Reference

!!! info "Latest Help"
    For the most current options: `bndbuild --help cpc`

## Usage

```bash
bndbuild --direct -- cpc <COMMAND> [OPTIONS]
```

## Commands

### `run`
Launch an emulator with the specified configuration.

### `orgams`
Special mode for Orgams assembler integration.

## Common Options

### `-a, --drivea <DISCA>`
Disc image to mount in drive A.

Supports: `.dsk`, `.edsk` formats.

### `-b, --driveb <DISCB>`
Disc image to mount in drive B.

### `--snapshot <SNAPSHOT>`
Snapshot file to load and run (`.sna` format).

### `-e, --emulator <EMULATOR>`
Choose which emulator to use.

Default: `ace`

Available emulators:
- `ace` - ACE emulator (cross-platform)
- `winape` - WinAPE (Windows)
- `cpcec` - CPCEC (cross-platform)
- `amspirit` - AMSpiriT (Windows)
- `sugarbox` - SugarboxV2 (cross-platform)

### `-m, --memory <MEMORY>`
Set CPC memory configuration (in KB).

Possible values: `64`, `128`, `192`, `256`, `320`, `576`, `1088`, `2112`

### `-k, --keepemulator`
Keep the emulator window open after execution completes.

Useful for:
- Interactive testing
- Debugging
- Manual exploration

### `-c, --clear-cache`
Clear the emulator download cache.

Use when:
- Emulator download is corrupted
- Forcing a fresh download
- Troubleshooting emulator issues

### `-d, --debug <DEBUG>`
Load debug symbols file (RASM-compatible format).

Currently supported by ACE emulator.

### `-r, --auto-run-file <AUTO_RUN_FILE>`
Automatically run the specified file from disk.

### `--albireo <FOLDER>`
Mount a folder as Albireo virtual filesystem (ACE only).

!!! warning
    This completely replaces the existing Albireo content.

### `--disable-rom <DISABLE_ROM>`
Disable specific ROMs.

Possible values:
- `orgams` - Disable Orgams ROM
- `unidos` - Disable UnidOS ROM

## Examples

See [Examples](examples.md) for practical usage scenarios.
