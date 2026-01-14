`basmdoc` aims at generating a HTML page that represents the documentation of a z80 assembler project.
As z80 source could we written on oldschool platform we have chosen to no use a verbose way to comment (such as `@param`).
Maybe we'll change that in the future if there are no users in for native code.

There are two kinds of comments:

- `;;;` represents a file comment
- `;;` represents a standard comment and serves to comment a following z80 token. Documentable tokens are:

   - labels
   - EQU
   - macro definitions
   - function definitions