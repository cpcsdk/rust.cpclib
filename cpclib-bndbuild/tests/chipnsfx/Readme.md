This example shows how to compile a chipnsfx music and run it on an emulator

- `bndbuild distclean` clears generated files
- `bndbuild CHIPNSFZ.BAS` builds the basic file to play `WINGSOD5.CHP`

   - if you add `-DSELECTED_MUSIC="OTHER.CHP"` you convert another music

- `bndbuild emu` launch the file on the `cpcec` emulator. It automatically builds `CHIPNSFZ.BAS` if absent

   - if you add `-DSELECTED_MUSIC="OTHER.CHP"` you convert another music
   - if you add `--DEMULATOR="OTHER"` you use OTHER (`ace`, `cpcec`, or `winape`) emulator

For example: `bndbuild emu -DEMULATOR=ace -DSELECTED_MUSIC="COLISEUM.CHP"`