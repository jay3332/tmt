#![feature(lint_reasons)]
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::{
    io::{stdout, Stdout},
    sync::mpsc::channel,
    time::Duration,
};
use tmt_core::{Component, ComponentType, Interface, Provider, TemperatureReading};

use ansi_to_tui::IntoText;
use crossterm::{
    cursor::Show,
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::CrosstermBackend as TuiBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

type BoxError = Box<dyn std::error::Error>;

macro_rules! exit {
    () => {{
        std::process::exit(0);
    }};
    ($code:literal) => {{
        std::process::exit($code);
    }};
}

#[allow(clippy::struct_excessive_bools, reason = "This is not a state machine")]
struct Options {
    interval: Duration,
    critical: f64,
    no_raw_mode: bool,
    summary: bool,
    vertical: bool,
}

fn option_parser() -> getopts::Options {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "print this help menu");
    opts.optflag("v", "version", "print the version");
    opts.optflag("N", "no-raw-mode", "do not enable raw terminal mode");
    opts.optflag("s", "summary", "hide details of individual components");
    opts.optflag("", "vertical", "optimize UI for vertical/tall terminals");
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
    let matches = opts.parse(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        exit!(2);
    });

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
        summary: matches.opt_present("s"),
        vertical: matches.opt_present("vertical"),
    })
}

type Backend = TuiBackend<Stdout>;

const HEADER: &str = concat!("TMT v", env!("CARGO_PKG_VERSION"));

macro_rules! key_value_ui {
    ($k:expr, $v:expr) => {{
        format!("{}{} {}\n", $k.bold().white(), ":".bold().white(), $v)
    }};
}

impl TemperatureReading for &Options {
    fn label(&self) -> String {
        unreachable!("options.label() should not be used in the UI")
    }

    fn temperature(&self) -> f64 {
        unreachable!("options.temperature() should not be used in the UI")
    }

    fn max(&self) -> f64 {
        unreachable!("options.max() should not be used in the UI")
    }

    fn high(&self) -> f64 {
        self.critical - 15.0
    }

    fn critical(&self) -> f64 {
        self.critical
    }
}

fn format_thermal_intensity(temp: f64, options: impl TemperatureReading) -> String {
    let mut reading = format!("{:.1}° C", temp);
    if temp >= options.critical() {
        reading = reading.red().bold().to_string();
        reading.push_str(" (CRITICAL)");
    } else if temp >= options.high() {
        reading = reading.yellow().bold().to_string();
    } else {
        reading = reading.green().bold().to_string();
    }
    reading
}

#[inline]
fn render_xpu<'a>(
    component_type: ComponentType,
    title: &'static str,
    name: String,
    show_all: bool,
    provider: &mut Provider,
    options: &'a Options,
) -> Option<Paragraph<'a>> {
    let components = provider.thermal_components_by_type(component_type);
    if components.is_empty() {
        return None;
    }

    let mut cpus_content = String::new();
    let (mut sum, mut total) = (0.0, 0_usize);
    let mut max = ("Unknown".to_string(), 0.0);

    for cpu in components.iter() {
        let temps = cpu.temperatures();

        for reading in temps {
            let temp = reading.temperature();
            sum += temp;
            total += 1;

            if temp > max.1 {
                max = (reading.label(), temp);
            }

            if show_all {
                cpus_content.push_str(&key_value_ui!(
                    reading.label(),
                    format_thermal_intensity(temp, reading)
                ));
            }
        }
    }

    let average = sum / total as f64;
    let mut cpus = format!("{} {}\n", "Name:".bold().cyan(), name.bold().white());
    cpus.push_str(&format!(
        "{} {}\n",
        "Count:".bold().cyan(),
        total.to_string().bold().white(),
    ));
    cpus.push_str(&format!(
        "{} {}\n",
        "Average:".bold().cyan(),
        format_thermal_intensity(average, options)
    ));
    cpus.push_str(&format!(
        "{} {} ({})\n",
        "Hottest:".cyan().bold(),
        max.0.white().bold(),
        format_thermal_intensity(max.1, options),
    ));
    cpus.push_str(&cpus_content);

    Some(
        Paragraph::new(cpus.into_text().unwrap())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .wrap(Wrap { trim: false }),
    )
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
        system.push_str(&key_value_ui!("Device", provider.device_model_name()));

        let system = Paragraph::new(system.into_text().unwrap()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("System")
                .border_style(Style::default().fg(Color::Gray)),
        );

        let entries = [
            render_xpu(
                ComponentType::Cpu,
                "CPUs",
                provider.cpu_name(),
                !options.summary,
                provider,
                options,
            ),
            render_xpu(
                ComponentType::Gpu,
                "GPUs",
                "N/A".to_string(),
                !options.summary,
                provider,
                options,
            ),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let constraints = if entries.is_empty() {
            Vec::with_capacity(0)
        } else {
            vec![Constraint::Percentage(100 / entries.len() as u16); entries.len()]
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
            .margin(1)
            .split(full);

        let next_row = Layout::default()
            .direction(if options.vertical {
                Direction::Vertical
            } else {
                Direction::Horizontal
            })
            .constraints(constraints)
            .split(layout[1]);

        frame.render_widget(block, full);
        frame.render_widget(system, layout[0]);

        for (i, entry) in entries.into_iter().enumerate() {
            frame.render_widget(entry, next_row[i]);
        }
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
    let provider = Provider::default();

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
        s.spawn(move || loop {
            if let Event::Key(key) = read().unwrap() {
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    esc_tx.send(()).unwrap();
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
