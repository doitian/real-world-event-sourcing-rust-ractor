use std::io::stderr;
use std::io::IsTerminal;

use anyhow::Result;
use tracing_glog::Glog;
use tracing_glog::GlogFields;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

pub fn init() -> Result<()> {
    let fmt = tracing_subscriber::fmt::Layer::default()
        .with_ansi(stderr().is_terminal())
        .with_writer(std::io::stderr)
        .event_format(Glog::default().with_timer(tracing_glog::LocalTime::default()))
        .fmt_fields(GlogFields::default().compact());

    let subscriber = Registry::default()
        .with(fmt)
        .with(EnvFilter::from_default_env());
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
