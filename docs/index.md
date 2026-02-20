!!! warning "Documentation Status"
    This documentation is **partially AI-generated** and may contain **hallucinations or inaccuracies**. 
    We are actively working to review and correct these issues. If you encounter incorrect information, 
    please report it at <https://github.com/cpcsdk/rust.cpclib/issues>.

cpclib is a [rust](https://www.rust-lang.org/) library that aims at helping to develop Amstrad CPC demos. 
Maybe it could be usefull for other z80 platform or even games. 
Most functionalities are still in beta state;
I have only implemented the subset I need for my current Amstrad CPC demo projects. 
Several tools are provided in addition to library.

There are more are less able to do:

- assemble z80 source code.
  - Mainly interesting for auto-generated code, not for handcrafted one.
  - Not all opcodes are managed.
  - Functionalities not available in other assemblers:
    - Injection of basic source code (WIP)
    - Function able to provided the opcode value of an instruction or its standard duration
- manipulate .sna files
  - Minimal support of chunks at the moment
- convert images to CPC format. Usable for standard resolutions/modes
- manipulate DSK (trying to mimick iDSK or dskmanager). Able to format and add files
- communicate with cpcwifi board
  - Replication of xfer utility.
  - Only reset and run file have been coded at the moment
  - In opposite to the original xfer tool, cpclib one is able to start snapshots V3 (there are simply converted as snapshot v2 on the fly)
- create basic tokens from basic source (WIP)
