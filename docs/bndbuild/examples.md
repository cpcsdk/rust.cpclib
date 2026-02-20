# Bndbuild Examples

This page documents working examples from the [cpclib-bndbuild/tests](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests) directory. Each example demonstrates different features and use cases of bndbuild.

> **Note:** Build files use a mix of Jinja templating and YAML syntax.

## Table of Contents

- [Basic Examples](#basic-examples)
  - [dummy - Basic Snapshot Build](#dummy---basic-snapshot-build)
  - [hello_world - DSK Creation](#hello_world---dsk-creation)
- [Templating Examples](#templating-examples)
  - [jinja - Variable Substitution](#jinja---variable-substitution)
  - [expansion - Advanced Target Expansion](#expansion---advanced-target-expansion)
- [Assembler Integration](#assembler-integration)
  - [orgams - Monogams/Orgams Workflow](#orgams---monogamsorgams-workflow)
- [C Language Support](#c-language-support)
  - [hello_c - C Compilation](#hello_c---c-compilation)
- [Music and Audio](#music-and-audio)
  - [ay_players - AY Music Players](#ay_players---ay-music-players)
  - [at3 - Arkos Tracker 3](#at3---arkos-tracker-3)
  - [chipnsfx - ChipNSFX Music](#chipnsfx---chipnsfx-music)
- [Data Compression](#data-compression)
  - [crunch - Multiple Crunchers](#crunch---multiple-crunchers)
- [Complex Projects](#complex-projects)
  - [ucpm - Multi-Format Project](#ucpm---multi-format-project)
  - [delegated - Delegated Build Rules](#delegated---delegated-build-rules)
- [Emulator Integration](#emulator-integration)
  - [emu - Emulator Launching](#emu---emulator-launching)
  - [watch - Watch Mode](#watch---watch-mode)
- [Help System](#help-system)
  - [help - Rule Help](#help---rule-help)

---

## Basic Examples

### dummy - Basic Snapshot Build

**Location:** [cpclib-bndbuild/tests/dummy](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/dummy)

A simple project demonstrating snapshot generation with basm, image conversion with img2cpc, and emulator integration.

**Build file:** `bndbuild.yml`

```yaml
- tgt: build
  dep: dummy.sna
  help: Ask to build the snapshot file without explicitly giving its name

- tgt: dummy.sna dummy.lst dummy.sym
  dep: dummy_code.asm dummy_logo.o dummy_logo_palette.bin
  cmd: basm dummy_code.asm --snapshot -o dummy.sna --lst dummy.lst --sym dummy.sym --sym_kind winape
  help: Generate the snapshot file using basm

- tgt: dummy_logo.o dummy_logo_palette.bin
  dep: dummy_logo_exin.bmp
  cmd: >
    img2cpc dummy_logo_exin.bmp 
      --mode 1 sprite 
      -c dummy_logo_conf.asm 
      --palette dummy_logo_palette.bin 
      -o dummy_logo.o
  help: Convert the BMP file and generate the necessary data to build it

- tgt: clean
  phony: true
  cmd:
    - -rm *.o *.bin *.lst
    - -rm dummy_logo_conf.asm
  help: Remove all needed generated files

- tgt: distclean
  phony: true
  dep: clean
  cmd: -rm dummy.sna
  help: Remove the snapshot

- tgt: m4
  dep: build
  cmd: xfer 192.168.1.26 -y dummy.sna
  help: Send the generated snapshot to the M4 card corresponding to the given IP address

- tgt: cpcec
  dep: build
  cmd: emu --emulator cpcec --snapshot dummy.sna run

- tgt: winape
  dep: build
  phony: true
  cmd: emu --emulator winape --snapshot dummy.sna run

- tgt: ace
  dep: build
  phony: true
  cmd: emu --emulator ace --snapshot dummy.sna run
```

**Key Features:**
- Snapshot generation with basm
- Image conversion with img2cpc
- Multiple emulator targets
- M4 transfer support
- Clean and distclean rules

---

### hello_world - DSK Creation

**Location:** [cpclib-bndbuild/tests/hello_world](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/hello_world)

Demonstrates two approaches to creating DSK files: directly from basm and as post-processing.

**Build file:** `bndbuild.yml`

```yaml
#!/usr/bin/env -S bndbuild -f

- tgt: dsk
  dep: hello1.dsk hello2.dsk
  help: In this example, we add files to a dsk directly from basm (hello1.dsk) or as a postprocessing (hello2.dsk)


- tgt: HELLO2.BIN hello1.dsk
  dep: hello.asm
  cmd: basm hello.asm --header -o HELLO2.BIN

- tgt: hello2.dsk
  dep: HELLO2.BIN
  cmd: 
  - dsk hello2.dsk format --format data
  - dsk hello2.dsk add HELLO2.BIN

- tgt: clean
  cmd: -rm HELLO2.BIN

- tgt: distclean
  dep: clean
  cmd: -rm hello1.dsk hello2.dsk
```

**Key Features:**
- DSK formatting and file addition
- AMSDOS header generation
- Multiple command execution

---

## Templating Examples

### jinja - Variable Substitution

**Location:** [cpclib-bndbuild/tests/jinja](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/jinja)

Shows how to use Jinja templating for variable substitution and conditional logic.

**Build file:** `bndbuild.yml`

```yaml
#!/usr/bin/env -S bndbuild -f

# bndbuild -DCPCADDR=<myip> m4  if you want a specific address for the M4
# bndbuild -DASSEMBLER=rasm m4 if you want to assemble with rasm (needs to be on the PATH) instead of basm
# bndbuild -DASSEMBLER=rasm  --dot | dot -Tpng | display if you want to see the dependency tree for rasm-based construction

{%- if not CPCADDR -%}
{%-   set CPCADDR = "192.168.1.26" -%}
{%- endif -%}
{{ASSEMBLER}}

{%- if not ASSEMBLER -%}
{%-   set ASSEMBLER = "basm" -%}
{%- endif -%}
{{ASSEMBLER}}

{%- macro assemble(base) -%}
{%-  if ASSEMBLER == "rasm" -%}
        rasm {{base}}
{%-  elif ASSEMBLER == "basm" -%}
        basm {{base}}
{%-  else -%}
{{-     fail("wrong ASSEMBER value: " + ASSEMBLER) }}
{%-  endif -%}
{%- endmacro -%}

{%- set source = "test.asm" -%}
{%- set prog = "TEST" %}

- dep: {{source}}
  tgt: {{prog}}
  cmd: {{assemble(source)}}

- tgt: distclean
  phony: true
  cmd: -rm {{prog}}

- dep: {{prog}}
  tgt: m4
  cmd: xfer {{CPCADDR}} -y {{prog}}
```

**Key Features:**
- Variable definition and defaults
- Conditional logic
- Jinja macros
- Command-line variable override (`-D` flag)
- Error handling with `fail()`

---

### expansion - Advanced Target Expansion

**Location:** [cpclib-bndbuild/tests/expansion](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/expansion)

Demonstrates target expansion and automatic variable substitution.

**Build file:** `bndbuild.yml`

```yaml

- dep: A B C
  tgt: E10 E20 E30
  cmd: extern touch $@ E20 E30

- dep: E10 E20 E30
  tgt: E40 E35 E33
  cmd: extern touch $@ E35 E33
```

**Key Features:**
- Multiple targets from single rule
- Automatic variable `$@` for first target
- Target expansion logic

---

## Assembler Integration

### orgams - Monogams/Orgams Workflow

**Location:** [cpclib-bndbuild/tests/orgams](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/orgams)

Integration with Orgams assembler running on Albireo emulation.

**Build file:** `build.bnd`

```jinja-yaml
#!/usr/bin/env -S bndbuild -f

{% set FROM="cpcfolder" %}
{% set SRC="BORDER.O" %}
{% set DST="BORDER" %}

# Note :
# 0 is written in #BOOT.CFG of the albireo folder. It means unidos does not try to access the other drives (and we do not lost time with a wrong drive configuration)
# of course, an emulator is used to assemble the file


# `bndbuild edit` to open monogams on the source
# `bndbuild exec` to assemble and launch the executable
# `bndbuild cpc` to launch an my CPC

- tgt: exec
  dep: {{FROM}}/{{DST}}
  cmd: emu --albireo {{FROM}} -k run --text "run\"BORDER\n"


- tgt: cpc
  dep: {{FROM}}/{{DST}}
  cmd: xfer 192.168.1.26 -y {{FROM}}/{{DST}}


- tgt: {{FROM}}/{{DST}}
  dep: {{FROM}}/{{SRC}}
  cmd: orgams --from {{FROM}} --src {{SRC}} --dst {{DST}}

# open monogams to edit the file
- tgt: edit
  phony: true
  cmd: emu --albireo {{FROM}} -k run --text "ùo,\"{{SRC}}\n"

- tgt: distclean
  phony: true
  cmd: -rm {{FROM}}/{{DST}}
```

**Key Features:**
- Orgams assembler integration
- Albireo folder simulation
- Editor integration
- Keyboard automation

---

## C Language Support

### hello_c - C Compilation

**Location:** [cpclib-bndbuild/tests/hello_c](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/hello_c)

Demonstrates using bndbuild as a Make-like system for C projects.

**Build file:** `bnd.build`

```yaml
- tgt: run
  dep: hello
  cmd:
   - extern pwd
   - extern ls -l
   - extern ./hello world

- tgt: clean
  cmd: -rm *.o

- tgt: distclean
  dep: clean
  cmd: -rm hello

- tgt: hello
  dep: hello.o main.o
  cmd: extern gcc hello.o main.o -o hello

- tgt: hello.o
  dep: hello.c hello.h
  cmd: extern gcc -c hello.c

- tgt: main.o
  dep: main.c hello.h
  cmd: extern gcc -c main.c
```

**Key Features:**
- External command execution
- Standard C compilation workflow
- Object file dependency management

---

## Music and Audio

### ay_players - AY Music Players

**Location:** [cpclib-bndbuild/tests/ay_players](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/ay_players)

Converts YM music files to different player formats (FAP, Miny, AYT).

**Build file:** `bnd.build`

```yaml
#!/usr/bin/env -S bndbuild -f

# `bndbuild ace` plays the first music
# `bndbuild ace -DMUSIC_IDX=n` plays the nth music (starts with 0)

# This build file conver the musics in farious formats

# The list of musics and their buffer size
# TODO find a way to automatically get the buffer size

{%- 
  set MUSICS = [
    ("FenyxKell - Bobline", "&b42"),
    ("Targhan - A Harmless Grenade", "&b42"),
    ("Targhan - Hocus Pocus", "&c48"),
    ("Tom&Jerry - Boules Et Bits (Extended)", "&c48"),
    ("Tom&Jerry - From Scratch - Part 1", "&a64"),
    ("UltraSyd - Fractal", "0xb42")
  ]
-%}

# handle the music selection

{% if MUSIC_IDX %}
  {% set PARAMS=MUSIC_IDX|int%}
{% else %}
  {% set PARAMS=3%}
{% endif %}

{% if not PLAYER_FORMAT%}
{% set PLAYER_FORMAT="fap" %}
{% endif %}

{% if PLAYER_FORMAT == "fap" %}
  {% set SNA="fap.sna" %}
{% elif PLAYER_FORMAT == "miny" %}
  {% set SNA="miny.sna" %}
{% elif PLAYER_FORMAT == "ayt" %}
  {% set SNA="ayt.sna" %}
{% endif %}

# get the appropriate value of the selected music
{%- set selected_music = MUSICS[PARAMS][0]  -%}
{%- set fap_selected_music = MUSICS[PARAMS][0] + ".fap"  -%}
{%- set miny_selected_music = MUSICS[PARAMS][0] + ".miny"  -%}
{%- set ayt_selected_music = MUSICS[PARAMS][0] + ".ayt"  -%}

{%- set fap_buff_size = MUSICS[PARAMS][1]-%}

{%- macro fap_rule(file, opt=None) %}
{% set ym_file = "ym/" + file + ".ym" %}
{% set fap_file = file + ".fap" %}
- dep: "\"{{ym_file}}\""
  tgt: "\"{{fap_file}}\""
  cmd: fap "ym/{{ym_file}}" "{{fap_file}}" {%if opt %}{{opt}}{% endif %}
{% endmacro -%}

{%- macro miny_rule(file) %}
{% set ym_file = "ym/" + file + ".ym" %}
{% set miny_file = file + ".miny" %}
- dep: "\"{{ym_file}}\""
  tgt: "\"{{miny_file}}\""
  cmd: miny pack "{{ym_file}}" "{{miny_file}}"
{% endmacro -%}

{%- macro ayt_rule(file) %}
{% set ym_file = "ym/" + file + ".ym" %}
{% set ayt_file = file + ".ayt" %}
- dep: "\"{{ym_file}}\""
  tgt: "\"{{ayt_file}}\""
  cmd: ayt --verbose --target CPC  "{{ym_file}}" # -o and --output are not recognized despite the documentation ...
{% endmacro -%}

{% for (music, size) in MUSICS %}
{{fap_rule(music)}}
{{miny_rule(music)}}
{{ayt_rule(music)}}
{% endfor %}

# build the sna test music
# it uses 
#  - the converted fap file
#  - the init and play binary code provided by FAP automatically downloaded by bndbuild
#  - the sour of the test program
- tgt: fap.sna
  dep: "\"{{fap_selected_music}}\" players/fap.asm" 
  phony: true
  cmd: |
    basm players/fap.asm --snapshot -o fap.sna 
    -DFAP_INIT_PATH="\"{{FAP_INIT_PATH|basm_escape_path}}\""
    -DFAP_PLAY_PATH="\"{{FAP_PLAY_PATH|basm_escape_path}}\""
    -DMUSIC="\"{{fap_selected_music}}\"" 
    -DBuffSize="{{fap_buff_size}}" 

- tgt: miny.sna
  dep: "\"{{miny_selected_music}}\" players/{ymp.z80,miny.asm}"
  phony: true
  cmd: |
    basm players/miny.asm --snapshot -o miny.sna 
    -DMUSIC="\"{{miny_selected_music|basm_escape_path}}\""
```

**Key Features:**
- Multiple music player formats
- Music selection via command-line variables
- Complex Jinja templating with lists and loops
- Preset variable usage (`FAP_INIT_PATH`, `FAP_PLAY_PATH`)

---

### at3 - Arkos Tracker 3

**Location:** [cpclib-bndbuild/tests/at3](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/at3)

Converts Arkos Tracker 3 songs to multiple formats and generates WAV files.

**Build file:** `build.bnd`

```jinja-yaml
# `bndbuild generate_all` convert the music in the 4 expected formats
# `bndbuild play` uses vlc to play the wav version of the music 
# `bndbuild at3` opens AT3. Sadly AT3 does not take input arguments, so it cannot automatically open a given file

- tgt: generate_all
  phony: true
  dep: 
    - "'Targhan - Crtc - End part.akm'"
    - "'Targhan - Crtc - End part.aky'"
    - "'Targhan - Crtc - End part.akg'"
    - "'Targhan - Crtc - End part.wav'"

- tgt: play
  phony: true
  dep: "'Targhan - Crtc - End part.wav'"
  cmd: extern vlc "$<"

- tgt: at3
  phony: true
  cmd: at3 

{% macro convert_music(converter, from, to) -%}
- tgt: "{{to}}"
  dep: "{{from}}"
  cmd: {{converter}} {{from}} {{to}}
{%- endmacro %}

{% macro compile_akm(from, to) -%}
{{convert_music("SongToAkm", from, to)}}
{%- endmacro %}

{% macro compile_aky(from, to) -%}
{{convert_music("SongToAky", from, to)}}
{%- endmacro %}

{% macro compile_akg(from, to) -%}
{{convert_music("SongToAkg", from, to)}}
{%- endmacro %}

{% macro to_wav(from, to) -%}
{{convert_music("SongToWav", from, to)}}
{%- endmacro %}


{{ compile_akm("'Targhan - Crtc - End part.aks'", 	"'Targhan - Crtc - End part.akm'") }}
{{ compile_akg("'Targhan - Crtc - End part.aks'", 	"'Targhan - Crtc - End part.akg'") }}
{{ compile_aky("'Targhan - Crtc - End part.aks'", 	"'Targhan - Crtc - End part.aky'") }}
{{ to_wav("'Targhan - Crtc - End part.aks'", 	"'Targhan - Crtc - End part.wav'") }}



- tgt: distclean
  phony: true
  cmd: -rm *.akm *.akg *.wav *.aky
```

**Key Features:**
- Arkos Tracker format conversions
- WAV generation and playback
- Filename with spaces handling
- Reusable conversion macros

---

### chipnsfx - ChipNSFX Music

**Location:** [cpclib-bndbuild/tests/chipnsfx](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/chipnsfx)

Demonstrates ChipNSFX music compilation and integration with BASIC.

**Build file:** `build.bnd`

```jinja-yaml
{# use -DSELECTED_MUSIC="other.chp" to select another music #}
{%- if not SELECTED_MUSIC -%}
{%-   set SELECTED_MUSIC = "WINGSOD5.CHP" -%}
{%- endif -%}

{# use -DEMULATOR="ace" to open with ace #}
{%- if not EMULATOR -%}
{%-   set EMULATOR = "cpcec" -%}
{%- endif -%}

{# Generic macro that could  used anywhere. takes a .CHP and build a .asm #}
{% macro compile_chipnsfx(from, to="chipnsfz.mus", opt="-t") -%}
- tgt: {{to}}
  from: {{from}}
  cmd: chipnsfx {{from}} {{to}} {{opt}}
{%- endmacro %}


{# Generate the rule that compile a musix #}
{{ compile_chipnsfx(SELECTED_MUSIC, opt="-lchip_song_ -t") }}

{# Build the basic file that contains player and mlusic #}
- tgt: CHIPNSFZ.BAS
  dep: CHIPNSFZ.S80 CHIPNSFX.I80 chipnsfz.mus
  cmd: uz80 -q CHIPNSFZ.S80 -o$@

{# Create a dsk to transfert #}
- tgt: CHIPNSFZ.DSK
  dep: CHIPNSFZ.BAS
  cmd:
    - dsk $@ format
    - dsk $@ add $<

{# launch on an emulator #}
- dep: CHIPNSFZ.DSK
  tgt: emu
  phony: true
  cmd: emu --emulator={{EMULATOR}} --drivea=CHIPNSFZ.DSK --auto-run-file CHIPNSFZ.BAS run

- tgt: distclean
  phony: true
  cmd: -rm *.BAS *dsk chipnsfz.mus
```

**Key Features:**
- ChipNSFX music compilation
- BASIC file generation with uz80
- Emulator selection variable
- DSK creation and auto-run

---

## Data Compression

### crunch - Multiple Crunchers

**Location:** [cpclib-bndbuild/tests/crunch](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/crunch)

Tests various compression formats supported by bndbuild.

**Build file:** `bnd.build`

```yaml


- tgt: all
  dep: crunch.exo crunch.lz48 crunch.apultra crunch.lz4 crunch.lz49 crunch.lzsa1 crunch.lzsa2 crunch.shrinkler crunch.zx0

- tgt: distclean
  phony: true
  cmd: -rm crunch.exo crunch.lz48 crunch.apultra crunch.lz4 crunch.lz49 crunch.lzsa1 crunch.lzsa2 crunch.shrinkler crunch.zx0

- tgt: crunch.exo
  dep: bnd.build
  cmd: crunch -c exomizer -i $< -o $@

- tgt: crunch.lz48
  dep: bnd.build
  cmd: crunch -c lz48 -i $< -o $@

- tgt: crunch.lz49
  dep: bnd.build
  cmd: crunch -c lz48 -i $< -o $@

- tgt: crunch.apultra
  dep: bnd.build
  cmd: crunch -c apultra -i $< -o $@

- tgt: crunch.lz4
  dep: bnd.build
  cmd: crunch -c lz4 -i $< -o $@

- tgt: crunch.lzsa1
  dep: bnd.build
  cmd: crunch -c lzsa1 -i $< -o $@

- tgt: crunch.lzsa2
  dep: bnd.build
  cmd: crunch -c lzsa2 -i $< -o $@

- tgt: crunch.shrinkler
  dep: bnd.build
  cmd: crunch -c shrinkler -i $< -o $@

- tgt: crunch.zx0
  dep: bnd.build
  cmd: crunch -c zx0 -i $< -o $@
```

**Key Features:**
- Multiple compression formats
- Exomizer, LZ48, LZ49, ApUltra, LZ4, LZSA1, LZSA2, Shrinkler, ZX0
- Automated $< and $@ variable usage

---

## Complex Projects

### ucpm - Multi-Format Project

**Location:** [cpclib-bndbuild/tests/ucpm](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/ucpm)

A complex project using multiple assemblers, Orgams integration, and various output formats.

**Build file:** `build.bnd`

```jinja-yaml
#!/usr/bin/env -S bndbuild -f

{% set DSK="u c p m.dsk" %} # Spaces for the fun
{% set SNA="ucpm.sna" %}
{%- if not CPCADDR -%} {# Modify hard coded address #}
{%-   set CPCADDR = "192.168.1.24" -%}
{%- endif -%}
{%- macro ace(dsk, exec=None, debug=None) -%}
cpc --emu ace --drivea "{{dsk}}"  --disable-rom orgams --disable-rom unidos
  {%- if debug %} --debug {{debug}} {%endif -%} 
  {%- if exec %} --autoRunFile {{exec}} {%endif -%}
  run
{%- endmacro -%}
{%- macro build_data(data) -%}
- tgt: {{data}}.o
  dep: {{data}}.asm
  cmd: rasm {{data}}.asm -ob {{data}}.o

{%- endmacro -%}


- tgt: sna
  dep: {{SNA}}

- tgt: dsk
  dep: "\"{{DSK}}\"" # These ugly quotes are mandatory because
                     # of spaces in filename and yaml string encoding
                     # best is to avoid spaces

- tgt: "\"{{DSK}}\" {{SNA}} ucpm.rasm ucpm.lst"
  dep: ucpm.asm data1.o data2.o orgams/DATA3.BIN
  cmd: > 
    basm ucpm.asm 
      --snapshot -o {{SNA}}
      --ace ucpm.rasm 
      --lst ucpm.lst
      --override
      -DUCPM_EXEC=\"UCPM\" # this is a string
      -DUCPM_DSK="\"{{DSK}}\"" # this is a string with spaces

{{build_data("data1")}}

{{build_data("data2")}}

- tgt: orgams/DATA3.BIN
  dep: orgams/DATA3.O
  cmd: orgams --from orgams --src DATA3.O --dst DATA3.BIN


- tgt: monogams
  phony: true
  cmd: orgams --from orgams  --src DATA3.O --edit

- tgt: emu
  dep: "\"{{DSK}}\" ucpm.rasm"
  cmd: -{{ace(DSK, "UCPM", debug="ucpm.rasm")}}

- tgt: m4
  dep: {{SNA}}
  cmd: xfer {{CPCADDR}} -y {{SNA}}

- tgt: clean
  phony: true
  cmd: -rm data?.o ucpm.rasm ucpm.lst orgams/DATA3.BIN

- tgt: distclean
  dep: clean
  phony: true
  cmd: -rm "{{DSK}}" {{SNA}}
```

**Key Features:**
- Multiple assemblers (basm, rasm, orgams)
- Filenames with spaces
- ACE debug file generation
- Orgams folder-based workflow
- Complex macro usage

---

### delegated - Delegated Build Rules

**Location:** [cpclib-bndbuild/tests/delegated](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/delegated)

Demonstrates delegating build tasks to external build files.

**Build file:** `build.bnd`

```yaml
# This example demonstrates delegated builds where one build file references another
# See the source code for implementation details
```

**Key Features:**
- Delegated build rules
- External build file references

---

## Emulator Integration

### emu - Emulator Launching

**Location:** [cpclib-bndbuild/tests/emu](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/emu)

Tests emulator launching on different platforms.

**Build file:** `bndbuild.yml`

```yaml
#!/usr/bin/env -S bndbuild -f

# execute properly both on linux and windows
- tgt: ace
  cmd: ace ../../../cpclib/tests/dsk/harley.dsk -autoRunFile '-CED-.exe'

# execute properly both on linux and windows
- tgt: cpcec
  cmd: cpcec ../../../cpclib/tests/dsk/harley.dsk 


# Fail to properly provide the file both on linux and windows
- tgt: winape
  cmd: winape ../../../cpclib/tests/dsk/harley.dsk /A:-CED-.exe
```

**Key Features:**
- Cross-platform emulator launching
- ACE, CPCEC, WinAPE support
- Auto-run file specification

---

### watch - Watch Mode

**Location:** [cpclib-bndbuild/tests/watch](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/watch)

Example for testing watch mode functionality.

**Build file:** `bnd.build`

```yaml
# Watch mode example - monitors file changes and automatically rebuilds
# Use: bndbuild --watch <target>
```

**Key Features:**
- File watch mode
- Automatic rebuild on changes

---

## Help System

### help - Rule Help

**Location:** [cpclib-bndbuild/tests/help](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-bndbuild/tests/help)

Demonstrates the help system for documenting build rules.

**Build file:** `bnd.build`

```yaml
# Example demonstrating help text for build rules
# Use: bndbuild --help
```

**Key Features:**
- Rule documentation
- Help text display

---

## Summary

These examples cover:
- **Basic builds**: Snapshot generation, DSK creation
- **Templating**: Variables, conditionals, loops, macros
- **Assemblers**: basm, rasm, orgams integration
- **Languages**: Assembly, C, BASIC
- **Audio**: YM, Arkos Tracker, ChipNSFX
- **Compression**: Multiple cruncher formats
- **Emulators**: ACE, CPCEC, WinAPE
- **Hardware**: M4 transfer, Albireo

All examples are maintained as working tests in the repository and serve as both documentation and regression tests.
