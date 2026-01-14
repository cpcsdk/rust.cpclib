# Sector Fight - Game Description

## Overview
*Sector Fight* is a turn-based strategy game developed in Locomotive BASIC 1.1 for the Amstrad CPC 6128. Two CPU-controlled players, CPU 1 and CPU 2, compete to dominate a grid by strategically placing and moving blocks. The game features customizable display modes, probabilistic battle mechanics, and personality-driven AI, offering a dynamic and replayable experience.

## Gameplay
- **Objective**: Players aim to control the most grid spaces by placing blocks or capturing opponent blocks through battles.
- **Grid Setup**: The game initializes a grid with dimensions based on the selected screen mode (Mode 0 or Mode 1), ensuring odd dimensions for balanced play.
- **Player Setup**: Each player starts with a set number of blocks, positioned randomly within designated grid zones (CPU 1 in the upper half, CPU 2 in the lower half).
- **Personalities**: Players are assigned one of four personalities (Normal, Attacker, Random, Defender) with equal probability (25% each), influencing their movement and combat decisions:
    - **Normal**: Balanced strategy.
    - **Attacker**: Prioritizes capturing opponent blocks.
    - **Random**: Makes unpredictable moves.
    - **Defender**: Focuses on maintaining control of existing blocks.
- **Turns**: Players alternate turns, selecting a block and moving to an adjacent empty space or engaging an opponent’s block. Moves are validated to ensure at least one adjacent valid move exists.
- **Battles**: When targeting an opponent’s block, a fight is resolved with a 50% chance of winning or losing, updating block counts and grid control accordingly.
- **Game End**: The game concludes when the grid is fully occupied, no valid moves remain, or one player loses all blocks. The winner is determined by the highest block count, with draws possible if counts are equal.

## Features
- **Screen Modes**: Players choose between Mode 0 (low resolution, more colors) or Mode 1 (higher resolution, fewer colors) at startup.
- **Auto-Pause**: Optional pause at the end of each turn for better visibility of moves (enabled via Y/N prompt).
- **Visuals**: Uses ASCII characters for blocks (CHR$(207) for players, CHR$(32) for empty spaces, CHR$(240) for highlights) with distinct colors for each player and text.
- **Sound Effects**: Includes basic sound effects for moves, battles, and grid initialization to enhance immersion.

## Technical Details
- **Language**: Locomotive BASIC 1.1.
- **Platform**: Amstrad CPC 6128.