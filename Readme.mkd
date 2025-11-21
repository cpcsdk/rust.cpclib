# cpclib


 ## Success stories

 Several demos have been released using this toolchain. However they may not build with the very last version due to its evolution.

 - Blight (2025) <https://www.pouet.net/prod.php?which=105251>, <https://github.com/rgiot/demo.bnd5.blight>
 - Amstrology (2025) <https://www.pouet.net/prod.php?which=104909>
 - 4deKades (2025) <https://www.pouet.net/prod.php?which=103970>, <https://github.com/rgiot/demo.revision2025.4deKades>
 - J'AI PÉ-TÉLÉCRAN (2024) <https://www.pouet.net/prod.php?which=96575>, <https://github.com/rgiot/demo.revision2024.etchy>
 - Come Join Us (2024) <https://www.pouet.net/prod.php?which=96537>
 - Can Robots Take Control? (2021) <https://www.pouet.net/prod.php?which=88554>

It is also able to build the (potentially closed, with minimal modifications) sources of

 - CRTC3 (2017) <https://www.pouet.net/prod.php?which=72279>
 - Goldorak <https://github.com/tbressel/goldorak-gx4000-beta/tree/master>
 
## Aim

`cpclib` is a library that aims at helping to develop Amstrad CPC demos.
Maybe it could be usefull for other `z80` platform or even games.
None of the functionalities are 100% functional. I have only implemented the subset I need for my current Amstrad CPC demo project.
Several tools are provided in addition to library.

There are more are less able to do:

 - assemble z80 source code. 
   * Mainly interesting for auto-generated code, not for handcrafted one.
   * Not all opcodes are managed.
   * Functionalities not available in other assemblers:
     - Injection of basic source code (WIP)
	 - Function able to provided the opcode value of an instruction or its standard duration
 - manipulate `.sna` files
   *  Minimal support of chunks at the moment
 - convert images to CPC format. Usable for standard resolutions/modes
 - manipulate DSK (trying to mimick iDSK or dskmanager). Able to format and add files
 - communicate with cpcwifi board
    * Replication of `xfer` utility.
	* Only reset and run file have been coded at the moment
	* In opposite to the original `xfer` tool, `cpclib` one is able to start snapshots V3 (there are simply converted as snapshot v2 on the fly)
 - create basic tokens from basic source (WIP)


 Documentation is available at <https://cpcsdk.github.io/rust.cpclib/>

 **WARNING** the windows toolchain has to be `nightly-x86_64-pc-windows-gnu`



