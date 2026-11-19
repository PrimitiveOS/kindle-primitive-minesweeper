use std::path::Path;

fn main() -> Result<(), String> {
    let dir = Path::new("/mnt/us/minesweeper");
    primitive_app::run(
        minesweeper_render::View::load(dir.join("minefield.json")),
        dir,
    )
}
