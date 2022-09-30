#![allow(clippy::cast_precision_loss)]

use std::{
    io::{stdout, Stdout, Write},
    sync::mpsc::channel,
    time::Duration,
};
use tmt_core::{Component, ComponentType, Interface, Provider};

use ansi_to_tui::IntoText;
use crossterm::{
    cursor::Show,
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    style::{Color as AnsiColor, ContentStyle, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::CrosstermBackend as TuiBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

type BoxError = Box<dyn std::error::Error>;

macro_rules! exit {
    () => {{
        std::process::exit(0);
    }};
}

struct Options {
    interval: Duration,
    critical: f64,
    no_raw_mode: bool,
}

fn option_parser() -> getopts::Options {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "print this help menu");
    opts.optflag("v", "version", "print the version");
    opts.optflag("N", "no-raw-mode", "do not enable raw terminal mode");
    opts.optopt(
        "i",
        "interval",
        "the interval, in seconds, between each data read",
        "SECONDS",
    );
    opts.optopt(
        "C",
        "critical",
        "the critical temperature threshold in celsius",
        "CELSIUS",
    );
    opts
}

fn parse_options() -> Result<Options, BoxError> {
    let opts = option_parser();
    let matches = opts.parse(std::env::args().skip(1))?;

    if matches.opt_present("h") {
        println!(
            "{}",
            opts.usage(
                "Usage: tmt [options]\nRun without options to start TMT (then press ESC to exit).",
            )
        );
        exit!();
    }

    if matches.opt_present("v") {
        println!("TMT v{}", env!("CARGO_PKG_VERSION"));
        exit!();
    }

    Ok(Options {
        interval: Duration::from_secs_f64(
            matches
                .opt_str("i")
                .unwrap_or_else(|| "2.0".to_string())
                .parse::<f64>()?,
        ),
        critical: matches
            .opt_str("C")
            .unwrap_or_else(|| "90.0".to_string())
            .parse::<f64>()?,
        no_raw_mode: matches.opt_present("N"),
    })
}

type Backend = TuiBackend<Stdout>;

const HEADER: &str = concat!("TMT v", env!("CARGO_PKG_VERSION"));

macro_rules! key_value_ui {
    ($k:expr, $v:expr) => {{
        format!("{}{} {}\n", $k.bold().white(), ":".bold().white(), $v)
    }};
}

fn format_thermal_intensity(temp: f64, options: &Options) -> String {
    let mut reading = format!("{:.1}° C", temp);
    if temp >= options.critical {
        reading = reading.red().bold().to_string();
        reading.push_str(" (CRITICAL)");
    } else if temp >= options.critical - 15.0 {
        reading = reading.yellow().bold().to_string();
    } else {
        reading = reading.green().bold().to_string();
    }
    reading
}

fn render(
    terminal: &mut Terminal<Backend>,
    provider: &mut Provider,
    options: &Options,
) -> Result<(), BoxError> {
    provider.refresh()?;

    terminal.set_cursor(0, 0)?;
    terminal.draw(|frame| {
        let size = frame.size();

        let full = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(size)[0];

        let block = Block::default()
            .title(HEADER)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let mut system = String::new();
        system.push_str(&key_value_ui!("Operating System", provider.os_name()));

        let system = Paragraph::new(system.into_text().unwrap()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("System")
                .border_style(Style::default().fg(Color::Gray)),
        );

        let mut cpus_content = String::new();
        let components = provider.thermal_components_by_type(ComponentType::Cpu);
        let mut sum = 0.0;
        let mut max = (0, 0.0);

        for (i, cpu) in components.iter().enumerate() {
            let temp = cpu.temperature();
            sum += temp;

            if temp > max.1 {
                max = (i, temp);
            }

            cpus_content.push_str(&key_value_ui!(
                cpu.label(),
                format_thermal_intensity(temp, options)
            ));
        }

        let average = sum / components.len() as f64;
        let mut cpus = format!(
            "{} {}\n",
            "Average:".cyan(),
            format_thermal_intensity(average, options)
        );
        cpus.push_str(&format!(
            "{} {} ({})\n",
            "Max:".cyan(),
            components[max.0].label().white().bold(),
            format_thermal_intensity(max.1, options),
        ));
        cpus.push_str(&cpus_content);

        let cpus = Paragraph::new(cpus.into_text().unwrap()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPUs")
                .border_style(Style::default().fg(Color::Gray)),
        );

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .margin(1)
            .split(full);

        frame.render_widget(block, full);
        frame.render_widget(system, layout[0]);
        frame.render_widget(cpus, layout[1]);
    })?;

    Ok(())
}

fn main() -> Result<(), BoxError> {
    let options = parse_options()?;

    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    if !options.no_raw_mode {
        enable_raw_mode()?;
    }

    let backend = TuiBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut provider = Provider::default();

    let (tx, rx) = channel();
    let esc_tx = tx.clone();
    let terminal = &mut terminal;

    std::thread::scope(|s| {
        s.spawn(|| {
            let tx = tx;
            let mut provider = provider;
            let options = options;

            loop {
                render(terminal, &mut provider, &options).unwrap_or_else(|err| {
                    eprintln!("Error occured while rendering: {}", err);
                    tx.send(()).unwrap();
                });
                std::thread::sleep(options.interval);
            }
        });
        s.spawn(|| {
            let tx = esc_tx;

            loop {
                if let Event::Key(key) = read().unwrap() {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Char('\x03') {
                        tx.send(()).unwrap();
                    }
                }
            }
        });
        s.spawn(move || {
            rx.recv().unwrap();
            disable_raw_mode().unwrap();
            execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show).unwrap();
            exit!();
        });
    });

    Ok(())
}
