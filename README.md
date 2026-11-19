# Primitive Minesweeper

Clear the 9 × 9, 16 × 16, or 16 × 30 minefield. Tap to reveal a square, or hold it to place a flag. **Flag mode**
lets you flag with a tap. The first revealed square and its neighbors are safe. Scroll larger boards with swipes or
the arrow buttons. The app keeps your game and best time at each size, and pauses the timer when the Kindle sleeps.

## Develop locally

Run `cargo test --offline --workspace` and `cargo run --offline -p minesweeper-desktop` from this directory, with the
Primitive Tools and Launcher repositories beside it. The desktop target runs sample touch, timer, and navigation
interactions and writes previews in `build/previews`.

See [INSTALL.md](INSTALL.md) and [device notes](docs/device-notes.md). `app.json` contains draft listing copy.
