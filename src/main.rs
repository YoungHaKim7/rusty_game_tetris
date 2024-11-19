use std::{
    error::Error,
    sync::mpsc,
    time::{Duration, Instant},
    {io, thread},
};

use crossterm::{
    cursor::{Hide, Show},
    event::{Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    {event, terminal, ExecutableCommand},
};
use rusty_audio::Audio;

use board::Board;

use crate::{
    frame::{new_frame, Drawable},
    score_board::ScoreBoard,
};

mod block;
mod board;
mod frame;
mod render;
mod score_board;

pub const SCREEN_NUM_ROWS: usize = 74;
pub const SCREEN_NUM_COLS: usize = 90;

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    for item in &["lose", "line", "rotate", "drop", "level_up"] {
        audio.add(item, format!("assets/audio/{}.wav", item));
    }

    let mut audio_background_music = Audio::new();
    for item in &["8bit_music_for_game_68698"] {
        audio_background_music.add(item, format!("assets/audio/{}.mp3", item));
    }

    // Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    // Render loop in a separate thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        while let Ok(_x) = render_rx.recv() {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
        // loop {
        // }
    });
    let mut board = Board::new(false); // pass true to display the memory map of the board
    let mut score_board =
        ScoreBoard::new(board.get_right_offset() + 5, board.get_bottom_offset() - 2);
    let mut instant = Instant::now();
    'game_loop: loop {
        // Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();
        audio_background_music.play("8bit_music_for_game_68698");

        // Input handlers for the game
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    // KeyCode::Esc | KeyCode::Char('q') => {
                    KeyCode::Esc => {
                        break 'game_loop;
                    }
                    KeyCode::Left => {
                        board.move_block_left();
                    }
                    KeyCode::Right => {
                        board.move_block_right();
                    }
                    KeyCode::Char(' ') => {
                        board.speed_up();
                    }
                    KeyCode::Up => {
                        audio.play("rotate");
                        board.rotate();
                    }
                    KeyCode::Down => board.speed_up(),
                    _ => {}
                }
            }
        }
        if board.update(delta) {
            audio.play("drop");
        }
        let lines = board.check_completed_lines();
        for _ in 0..lines {
            audio.play("line");
        }
        if score_board.update(lines) {
            audio.play("level_up");
            board.increase_speed();
        }
        board.draw(&mut curr_frame);
        score_board.draw(&mut curr_frame);
        if board.board_is_full() {
            audio.play("lose");
            break 'game_loop;
        }

        let _ = render_tx.send(curr_frame);
        thread::sleep(Duration::from_millis(1));
    }
    // Terminal
    // Cleanup
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
