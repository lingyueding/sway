//! Utility items shared between forc crates.

use ansi_term::Colour;
use std::env;
use std::io;
use std::str;
use tracing::{Level, Metadata};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::MakeWriter,
};

pub fn println_red(txt: &str) {
    println_std_out(txt, Colour::Red);
}

pub fn println_green(txt: &str) {
    println_std_out(txt, Colour::Green);
}

pub fn println_yellow_err(txt: &str) {
    println_std_err(txt, Colour::Yellow);
}

pub fn println_red_err(txt: &str) {
    println_std_err(txt, Colour::Red);
}

fn println_std_out(txt: &str, color: Colour) {
    tracing::info!("{}", color.paint(txt));
}

fn println_std_err(txt: &str, color: Colour) {
    tracing::error!("{}", color.paint(txt));
}

// This allows us to write ERROR and WARN level logs to stderr and everything else to stdout.
// https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
struct StdioTracingWriter {
    writer_mode: TracingWriterMode,
}

impl<'a> MakeWriter<'a> for StdioTracingWriter {
    type Writer = Box<dyn io::Write>;

    fn make_writer(&'a self) -> Self::Writer {
        if self.writer_mode == TracingWriterMode::Stderr {
            Box::new(io::stderr())
        } else {
            // We must have an implementation of `make_writer` that makes
            // a "default" writer without any configuring metadata. Let's
            // just return stdout in that case.
            Box::new(io::stdout())
        }
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        // Here's where we can implement our special behavior. We'll
        // check if the metadata's verbosity level is WARN or ERROR,
        // and return stderr in that case.
        if self.writer_mode == TracingWriterMode::Stderr
            || (self.writer_mode == TracingWriterMode::Stdio && meta.level() <= &Level::WARN)
        {
            return Box::new(io::stderr());
        }

        // Otherwise, we'll return stdout.
        Box::new(io::stdout())
    }
}

#[derive(PartialEq, Eq)]
pub enum TracingWriterMode {
    /// Write ERROR and WARN to stderr and everything else to stdout.
    Stdio,
    /// Write everything to stdout.
    Stdout,
    /// Write everything to stderr.
    Stderr,
}

#[derive(Default)]
pub struct TracingSubscriberOptions {
    pub verbosity: Option<u8>,
    pub silent: Option<bool>,
    pub log_level: Option<LevelFilter>,
    pub writer_mode: Option<TracingWriterMode>,
    pub ansi: Option<bool>,
    pub display_time: Option<bool>,
}

/// A subscriber built from default `tracing_subscriber::fmt::SubscriberBuilder` such that it would match directly using `println!` throughout the repo.
///
/// `RUST_LOG` environment variable can be used to set different minimum level for the subscriber, default is `INFO`.
pub fn init_tracing_subscriber(options: TracingSubscriberOptions) {
    // Parse the log level from the options, if set.
    let level_filter = options.log_level.or({
        match options.verbosity {
            Some(1) => Some(LevelFilter::DEBUG), // matches --verbose or -v
            Some(2) => Some(LevelFilter::TRACE), // matches -vv
            _ => None,
        }
    });

    // Use the log level from options if provided, otherwise use the RUST_LOG setting.
    let env_filter = level_filter
        .map(|level_filter| {
            // If silent is set, we want to disable all logs.
            if options.silent.unwrap_or_default() {
                return EnvFilter::new(LevelFilter::OFF.to_string());
            }

            // The options level filter only applies to packages prefixed with `forc`, `sway`, or `test`. This is to filter out
            // noisy logs from dependencies. To get all logs, use `RUST_LOG=trace`.
            let env_log_level = env::var("RUST_LOG").unwrap_or(LevelFilter::INFO.to_string());
            EnvFilter::builder().parse_lossy(format!(
                "{},forc={},sway={},test={}",
                env_log_level, level_filter, level_filter, level_filter
            ))
        })
        .unwrap_or_else(|| EnvFilter::builder().from_env_lossy());

    let builder = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_ansi(options.ansi.unwrap_or_default())
        .with_level(false)
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .with_writer(StdioTracingWriter {
            writer_mode: options.writer_mode.unwrap_or(TracingWriterMode::Stdio),
        });

    if options.display_time.unwrap_or_default() {
        builder.init();
    } else {
        builder.without_time().init();
    }
}
