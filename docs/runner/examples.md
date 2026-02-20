# CPC Runner Examples

## Basic Usage

### Run a Snapshot
```bash
bndbuild --direct -- cpc run --snapshot demo.sna
```

### Run with Disk Image
```bash
bndbuild --direct -- cpc run -a game.dsk
```

### Run with Two Disks
```bash
bndbuild --direct -- cpc run -a disk1.dsk -b disk2.dsk
```

## Choosing Emulators

### Use WinAPE (Windows)
```bash
bndbuild --direct -- cpc run -a game.dsk -e winape
```

### Use ACE (Default)
```bash
bndbuild --direct -- cpc run -a game.dsk -e ace
```

### Use CPCEC
```bash
bndbuild --direct -- cpc run --snapshot demo.sna -e cpcec
```

## Memory Configuration

### 128KB CPC
```bash
bndbuild --direct -- cpc run -a game.dsk -m 128
```

### 64KB CPC 464
```bash
bndbuild --direct -- cpc run -a game.dsk -m 64
```

## Development Workflow

### Keep Emulator Open for Testing
```bash
bndbuild --direct -- cpc run -a test.dsk -k
```

### Load Debug Symbols
```bash
bndbuild --direct -- cpc run -a demo.dsk -d demo.sym -e ace
```

### Auto-run Specific File
```bash
bndbuild --direct -- cpc run -a disk.dsk -r DEMO.BIN
```

## Albireo Virtual Filesystem (ACE)

### Mount Local Folder
```bash
bndbuild --direct -- cpc run --albireo ./files/ -e ace
```

Files in the `./files/` folder will be accessible from the CPC.

## Integration Examples

### In bndbuild.yml - Test Task
```yaml
- tgt: test
  dep: demo.sna
  cmd: cpc run --snapshot demo.sna -k
  phony: true
```

### Run with Specific Memory
```yaml
- tgt: run128
  dep: game.dsk
  cmd: cpc run -a game.dsk -m 128 -k
  phony: true
```

### Multi-disk Demo
```yaml
- tgt: run-demo
  dep: [demo1.dsk, demo2.dsk]
  cmd: cpc run -a demo1.dsk -b demo2.dsk -e ace
  phony: true
```

## Troubleshooting

### Clear Emulator Cache
```bash
bndbuild --direct -- cpc run -c
```

### Force ACE Download
```bash
# Clear cache then run
bndbuild --direct -- cpc run -c -a game.dsk -e ace
```

## Advanced Scenarios

### Disable Orgams ROM
```bash
bndbuild --direct -- cpc run -a disk.dsk --disable-rom orgams
```

### Development Cycle
```bash
# Build, then run with debugging
bndbuild build && bndbuild --direct -- cpc run -a output.dsk -d output.sym -k
```

### Cross-emulator Testing
```bash
# Test on multiple emulators
for emu in ace winape cpcec; do
  bndbuild --direct -- cpc run -a demo.dsk -e $emu
done
```
