# Sector Fight Code Reference

**Filename**: `docs/sector_fight_reference.md`

This document provides a detailed reference for the structure and variables of *Sector Fight*, a turn-based strategy game written in Locomotive BASIC 1.1 for the Amstrad CPC 6128. It is intended to assist future contributors in understanding the codebase, focusing on global variables and program structure, with a brief section on local variables. Line numbers are omitted as they may change with updates.

## Program Structure

The game is structured as a single BASIC program with a main game loop and modular subroutines for handling specific functionalities. Below is an overview of the key sections:

- **Initialization**:
    - Prompts user for screen mode (0 or 1) and auto-pause option (Y/N).
    - Configures display settings (colors, border, paper, pen, mode).
    - Defines symbols for blocks and highlights.
    - Initializes arrays for player stats, battle probabilities, and personalities.
    - Assigns random personalities to CPU players.
    - Displays a "Press any key to start" message.

- **Grid and Player Setup**:
    - Calculates grid dimensions based on screen mode, ensuring odd width/height for balance.
    - Draws grid borders using ASCII characters.
    - Places initial blocks for CPU 1 (upper half) and CPU 2 (lower half).
    - Initializes player stats (position, count, min/max coordinates).
    - Builds lists of valid moves for each player.

- **Main Game Loop**:
    - Alternates turns between CPU 1 and CPU 2 until the grid is full or no valid moves remain.
    - Displays turn number, progress percentage, and block counts.
    - Processes CPU actions based on personality (currently, all use Random logic).
    - Handles moves (to empty spaces) or battles (against opponent blocks).
    - Updates grid, stats, and valid move lists after each action.
    - Plays sound effects for moves and battle outcomes.
    - Optionally pauses after each turn if auto-pause is enabled.
    - Ends with a "Game Over" message, block counts, and winner/draw declaration.
    - Prompts to play again (Y/N).

- **Subroutines**:
    - **Personality Handlers**: Normal, Attacker, Random, Defender (currently, Normal, Attacker, and Defender call Random).
    - **Action Handler**: Processes moves or resolves battles (50% win/loss chance).
    - **Grid Management**: Updates stats, recalculates min/max coordinates, and manages valid move lists.
    - **Display Functions**: Draws grid, highlights blocks, prints messages, and updates block counts.
    - **Error Handling**: Displays error codes and messages, then exits gracefully.
    - **Utility Functions**: Validates moves, removes blocks from lists, and centers text.

- **Data Section**:
    - Defines stat labels (e.g., "start", "sum", "avg").
    - Lists personality names (full names for Mode 1, shorthands for Mode 0).

## Global Variables

Below is a comprehensive list of global variables, grouped by purpose, with descriptions of their roles in the game.

### Game Configuration
- `smd`: Screen mode (0 or 1), set by user input, affects resolution and colors.
- `ps`: Auto-pause flag (0 = no, 1 = yes), pauses after each turn if enabled.
- `cols`, `rows`: Screen dimensions in characters (based on `smd`).
- `hcols`, `hrows`: Half of `cols` and `rows`, used for centering text.

### Display and Colors
- `cbg`: Background color (0 = black).
- `cl1`, `cl2`: Colors for CPU 1 and CPU 2 blocks (1 = red, 2 = green).
- `ctx`: Text color (3 = white).
- `b1$`, `b2$`: Block symbols for CPU 1 and CPU 2 (CHR$(207)).
- `eb$`: Empty block symbol (CHR$(32)).
- `hb$`: Highlight symbol (CHR$(240), custom-defined).

### Player Identification
- `id1`, `id2`: Player IDs (1 for CPU 1, 2 for CPU 2).
- `id1$`, `id2$`: Player names ("CPU 1", "CPU 2").
- `c1`, `c2`: Block counts for CPU 1 and CPU 2.
- `c1$`, `c2$`: Status strings (name + personality) for display.
- `c1x`, `c1y`, `c2x`, `c2y`: Screen coordinates for printing player status.

### Grid and Positioning
- `gw`, `gh`: Grid width and height, calculated to be odd and fit screen.
- `gwh`: Total grid spaces (`gw * gh`).
- `hgw`, `hgh`: Half of `gw` and `gh`, for centering.
- `ofx`, `ofy`: Grid offset coordinates for centering on screen.
- `grd(gw,gh)`: 2D array representing the grid (0 = empty, 1 = CPU 1, 2 = CPU 2, -1 = border).
- `st1x`, `st1y`, `st2x`, `st2y`: Starting coordinates for CPU 1 and CPU 2.

### Player Stats
- `ial`: Number of stat types (7: start, sum, avg, min, max, selected, last, count).
- `ist`, `ism`, `ivg`, `imn`, `imx`, `isl`, `ilt`, `icn`: Indices for stat types (0 to 7).
- `st(2,ial,1)`: 3D array for player stats (2 players, 8 stat types, 2 coordinates: x and y).
    - `st(id,ist,0/1)`: Starting x/y position.
    - `st(id,ism,0/1)`: Sum of x/y coordinates of all blocks.
    - `st(id,ivg,0/1)`: Average x/y coordinates.
    - `st(id,imn,0/1)`: Minimum x/y coordinates.
    - `st(id,imx,0/1)`: Maximum x/y coordinates.
    - `st(id,isl,0/1)`: Last selected block x/y.
    - `st(id,ilt,0/1)`: Last occupied block x/y.
    - `st(id,icn,0)`: Block count (y unused).
- `st$(ial)`: Stat type names ("start", "sum", "avg", "min", "max", "sel", "last", "count").

### Battle Mechanics
- `bsz`: Size of battle probability array (8).
- `frn`, `frx`, `fra`: Indices for friendly block min/max/avg probabilities (0.05, 0.1, 0.075).
- `opn`, `opx`, `opa`: Indices for opposing block min/max/avg probabilities (-0.1, -0.05, -0.075).
- `emn`, `emx`, `ema`: Indices for empty block min/max/avg probabilities (0.01, -0.03, -0.01).
- `btl(bsz)`: Array of battle probabilities (currently unused in favor of 50% win/loss).
- `attthres`, `defthres`: Attack and defense thresholds (0.3, unused).

### Personality Mechanics
- `psz`: Number of personalities (3: Normal, Attacker, Random, Defender).
- `pnrm`, `patt`, `prnd`, `pdef`: Personality indices (0 to 3).
- `pnprb(psz)`: Cumulative personality probabilities (0.25 each, totaling 1.0).
- `pn$(psz)`: Personality names ("Normal", "Attacker", "Random", "Defender" in Mode 1; "Nrm", "Att", "Rnd", "Def" in Mode 0).
- `pn1`, `pn2`: Assigned personalities for CPU 1 and CPU 2.

### Move Management
- `blmax`: Maximum valid moves per player (`(gw + gh) * 2`).
- `bls(1,blmax,1)`: 3D array of valid moves (0 = CPU 1, 1 = CPU 2; 0,0 = count; x,y coordinates).
- `vm(8,1)`: Array of up to 8 adjacent valid moves for a block (0,0 = count; x,y coordinates).

### Game State
- `turn`: Current turn (1 = CPU 1, 2 = CPU 2).
- `trn`: Turn counter.
- `trs`: Turn status flag (1 = turn completed).
- `prg`: Grid fill percentage.
- `act`: Action result (0 = no move, 1 = move, 2 = fight won, 3 = fight lost).

### Display and Input
- `ms$`: Message string for centered display (e.g., "Loading...", "Game Over").
- `sx`, `sy`: Status line coordinates.
- `a$`: User input string (for mode, pause, replay).

## Local Variables

Local variables are primarily temporary and used within subroutines for calculations, loop counters, or intermediate results. They are less critical for understanding the overall codebase but are listed below for completeness. These variables are typically reused across subroutines, so their scope is limited to the subroutine's execution.

- `tmp`: Temporary counter or result (e.g., valid move count, block count).
- `tmpx`, `tmpy`: Temporary x/y coordinates for move validation or display.
- `tmpid`, `tmpopp`: Temporary player IDs for subroutine processing.
- `r`: Random value for personality, move, or battle outcomes.
- `i`, `j`: Loop counters for grid scanning or array iteration.
- `dx`, `dy`: Delta x/y for checking adjacent grid spaces.
- `nx`, `ny`: Next x/y coordinates for move validation.
- `bx`, `by`: Selected block x/y coordinates.
- `tx`, `ty`: Target x/y coordinates for a move.
- `hx`, `hy`: Highlight x/y coordinates.
- `minx`, `maxx`, `miny`, `maxy`: Temporary min/max coordinates for stat updates.
- `cpuclr`: Temporary color for drawing blocks or highlights.
- `mst$`: Temporary status message (e.g., "...").
- `sb`: Starting block count based on grid size.
- `p`: Percentage for starting blocks (0.1).
- `ids`: Player index (0 for CPU 1, 1 for CPU 2) in move lists.

These variables are transient and should be carefully checked for conflicts when modifying subroutines, as BASIC does not enforce strict scoping.

## Notes
- **Variable Naming**: Variables are concise due to BASIC’s limitations, but grouped logically (e.g., `cl1`, `cl2` for colors; `st1x`, `st2x` for starting positions).
- **Array Usage**: Arrays like `st`, `bls`, and `vm` are critical for game state; ensure indices (`ial`, `bsz`, etc.) are respected.
- **Personality Logic**: Currently, only Random personality is fully implemented; enhance Normal, Attacker, and Defender for strategic variety.
- **Battle System**: The `btl` array and thresholds (`attthres`, `defthres`) are defined but unused; integrate them for more nuanced battles.
- **Error Handling**: Subroutine at `3010` handles errors; ensure new code updates `ms$` for clear messaging.
- **Display Constraints**: Screen dimensions (`cols`, `rows`) limit grid size; test changes across both screen modes.
