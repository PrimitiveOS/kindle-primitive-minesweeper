use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use minesweeper_core::{BestTimes, Difficulty, Minefield, Status, View as Tile};
use minesweeper_render::View;
use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&out)?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut app = View::new(
        Minefield::new(Difficulty::Beginner, 42),
        BestTimes::default(),
        None,
    );
    write_png(
        &out.join("minesweeper.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(170.0, 370.0)), Change::Partial);
    assert_eq!(app.game().view(0, 0), Tile::Open(0));
    assert_eq!(app.game().status(), Status::Playing);
    assert_eq!(
        app.tick(Instant::now() + Duration::from_secs(7)),
        Change::Partial
    );
    assert_eq!(app.input(Input::LongTap(1065.0, 1208.0)), Change::Partial);
    assert_eq!(app.game().view(8, 8), Tile::Flag);
    write_png(
        &out.join("minesweeper-game-dark.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    assert_eq!(app.input(Input::Tap(650.0, 1560.0)), Change::Full);
    write_png(
        &out.join("minesweeper-best.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(650.0, 1110.0)), Change::Full);
    assert_eq!(app.input(Input::Tap(1060.0, 1560.0)), Change::Full);
    write_png(
        &out.join("minesweeper-new.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(640.0, 950.0)), Change::Full);
    assert_eq!(app.game().difficulty(), Difficulty::Expert);
    assert_eq!(
        app.input(Input::Swipe {
            dx: -300.0,
            dy: 0.0,
            start_y: 400.0
        }),
        Change::Full
    );
    assert_eq!(app.origin(), (0, 10));
    write_png(
        &out.join("minesweeper-expert.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    Ok(())
}
