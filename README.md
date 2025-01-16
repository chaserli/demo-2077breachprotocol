# Rust Breach Protocol

A Rust + macroquad take on Cyberpunk 2077's Breach Protocol minigame.

## Play

Pick tokens from the code matrix to fill the buffer and upload daemon sequences.
The first pick must come from the top row. After each pick, the allowed line
alternates between the selected column and selected row. A cell can only be used
once.

Daemon sequences upload when their tokens appear contiguously in the buffer.
Multiple daemons can upload in one run, including overlapping sequences.
