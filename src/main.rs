mod character;
mod render;
mod sdf;

use character::Anim;
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use render::ColorMode;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// A tiny 3D character that lives in your terminal.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Animation to play.
    #[arg(short, long, value_enum, default_value_t = Anim::Auto)]
    anim: Anim,

    /// Frames per second.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=120))]
    fps: u32,

    /// Animation speed multiplier.
    #[arg(long, default_value_t = 1.0)]
    speed: f32,

    /// Stop after this many seconds (runs until q/Esc otherwise).
    #[arg(short, long)]
    duration: Option<f32>,

    /// Colour output (auto-detected by default).
    #[arg(long, value_enum)]
    color: Option<ColorMode>,

    /// Shortcut for --color none.
    #[arg(long)]
    mono: bool,

    /// Print a single frame to stdout and exit (no animation).
    #[arg(long)]
    once: bool,

    /// Time in seconds to render with --once.
    #[arg(long, default_value_t = 1.0)]
    time: f32,

    /// Width in cells (defaults to the terminal width).
    #[arg(long)]
    width: Option<u16>,

    /// Height in cells (defaults to the terminal height).
    #[arg(long)]
    height: Option<u16>,
}

impl Args {
    fn color_mode(&self) -> ColorMode {
        if self.mono {
            ColorMode::None
        } else {
            self.color.unwrap_or_else(ColorMode::detect)
        }
    }

    fn size(&self) -> (usize, usize) {
        let (tw, th) = terminal::size().unwrap_or((80, 40));
        (
            self.width.unwrap_or(tw) as usize,
            self.height.unwrap_or(th) as usize,
        )
    }
}

/// Restores the terminal even if we exit early or panic.
struct TermGuard;

impl TermGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(terminal::ClearType::All)
        )?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn is_quit(k: &KeyEvent) -> bool {
    matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
        || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL))
}

fn run_once(args: &Args) -> io::Result<()> {
    let (w, h) = args.size();
    let mode = args.color_mode();
    let pose = character::pose(args.anim, args.time);
    let cells = render::render(w, h, &pose, mode == ColorMode::None);
    let mut out = io::stdout().lock();
    writeln!(out, "{}", render::encode(&cells, w, mode, "\n"))
}

fn run_live(args: &Args) -> io::Result<()> {
    let mode = args.color_mode();
    let mono = mode == ColorMode::None;
    let frame_time = Duration::from_secs_f64(1.0 / args.fps as f64);
    let _guard = TermGuard::enter()?;
    let mut out = io::stdout().lock();
    let start = Instant::now();
    let mut last_size = (0, 0);

    loop {
        let frame_start = Instant::now();
        let elapsed = start.elapsed().as_secs_f32();
        if args.duration.is_some_and(|d| elapsed >= d) {
            break;
        }

        let (w, h) = args.size();
        if (w, h) != last_size {
            execute!(out, terminal::Clear(terminal::ClearType::All))?;
            last_size = (w, h);
        }
        let pose = character::pose(args.anim, elapsed * args.speed);
        let cells = render::render(w, h, &pose, mono);
        let frame = render::encode(&cells, w, mode, "\r\n");
        out.write_all(b"\x1b[H")?;
        out.write_all(frame.as_bytes())?;
        out.flush()?;

        // Handle input until it's time for the next frame.
        let deadline = frame_start + frame_time;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || !event::poll(left)? {
                break;
            }
            match event::read()? {
                Event::Key(k) if is_quit(&k) => return Ok(()),
                Event::Resize(..) => last_size = (0, 0),
                _ => {}
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    if args.once {
        run_once(&args)
    } else {
        run_live(&args)
    }
}
