use anyhow::Context;
use indicatif::*;
use is_terminal::*;
use serde::*;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Display,
    path::*,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use structopt::*;

mod cache;
mod file_selector;
mod hash_files;

mod multi_runner;
use multi_runner::*;

mod targets;
use targets::*;

#[derive(StructOpt)]
struct Options {
    #[structopt(long, default_value = "./gentle.toml")]
    config_file: PathBuf,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
pub enum Command {
    CacheLoad {
        from: PathBuf,
    },
    CacheSave {
        to: PathBuf,
    },
    LockfileHash,

    Do(ActionCommand),

    // TODO(shelbyd): Allow multiple actions.
    #[structopt(flatten)]
    Action(Action),
}

#[derive(Debug, PartialEq, Eq, Clone, StructOpt)]
pub struct ActionCommand {
    #[structopt(long)]
    filter: Option<String>,

    #[structopt(subcommand)]
    verb: Action,
}

#[derive(Debug, PartialEq, Eq, Clone, StructOpt)]
pub enum Action {
    Test,
    Lint,
    Format,
    Build(Build),
}

#[derive(Debug, PartialEq, Eq, Clone, StructOpt)]
pub struct Build {
    #[structopt(
        long,
        help = "Directory to write outputs to",
        default_value = "gentle/out"
    )]
    out: PathBuf,
}

impl Action {
    fn can_cache_success(&self) -> bool {
        match self {
            Action::Test | Action::Lint | Action::Format => true,
            Action::Build(_) => false,
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Test => write!(f, "test"),
            Action::Lint => write!(f, "lint"),
            Action::Format => write!(f, "format"),
            Action::Build(_) => write!(f, "build"),
        }
    }
}

#[derive(Deserialize, Default)]
struct Config {
    skip: HashSet<String>,
}

fn main() -> anyhow::Result<()> {
    let options = Options::from_args();

    let config = if let Ok(file) = std::fs::read(&options.config_file) {
        toml::from_slice(&file)?
    } else {
        Config::default()
    };

    let action = match options.command {
        Command::CacheLoad { from } => return cache::load(from),
        Command::CacheSave { to } => return cache::save(to),
        Command::LockfileHash => {
            let files = targets()?.into_iter().flat_map(|t| t.lock_files());
            println!("{}", hash_files::hash_files(files)?.to_hex());
            return Ok(());
        }
        Command::Do(action) => action,
        Command::Action(verb) => ActionCommand { verb, filter: None },
    };

    let targets = targets::targets()?
        .into_iter()
        .filter(|t| should_run(&t.to_string(), &config.skip, &action.filter))
        .collect::<Vec<_>>();

    let progress: Box<dyn ProgressListener> = if std::env::var("CI") == Ok(String::from("true")) {
        Box::new(ContinuousIntegrationProgress::new(targets.len()))
    } else if std::io::stderr().is_terminal() {
        Box::new(TermProgress::new())
    } else {
        Box::new(NullProgressListener)
    };
    let mut runner = ParRunner::new(progress);

    for target in targets {
        let action = action.clone();
        runner
            .run(&format!("{} {target}", action.verb), move || {
                maybe_cache_success(&action.verb, &*target, || match &action.verb {
                    Action::Test => target.perform_test(),
                    Action::Lint => target.perform_lint(),
                    Action::Format => target.perform_format(),
                    Action::Build(build) => target.perform_build(build),
                })
            })
            .map_err(|(id, err)| err.context(id))?;
    }
    runner.into_wait().map_err(|(id, err)| err.context(id))?;

    Ok(())
}

fn should_run(target: &str, skip: &HashSet<String>, filter: &Option<String>) -> bool {
    if skip.contains(target) {
        return false;
    }

    if let Some(filter) = filter {
        if !target.starts_with(filter) {
            return false;
        }
    }

    true
}

fn maybe_cache_success(
    action: &Action,
    target: &dyn Target,
    f: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if !action.can_cache_success() {
        return f();
    }

    let files = match target.src_files()? {
        None => return f(),
        Some(f) => f,
    };

    let files = files.list().context("Listing files")?;
    let hash = hash_files::hash_files(files)
        .context("Hashing files")?
        .to_hex();
    let cache_path = PathBuf::from(format!(".gentle_cache/successes/{hash}"));

    if cache_path.exists() {
        return Ok(());
    }

    f()?;

    std::fs::create_dir_all(cache_path.parent().expect("explicit subdirectory"))?;
    std::fs::File::create(cache_path).context("Creating cache marker")?;

    Ok(())
}

struct TermProgress {
    multi: MultiProgress,
    bars: Vec<(ProgressBar, Option<String>)>,
}

impl TermProgress {
    fn new() -> Self {
        TermProgress {
            multi: MultiProgress::new(),
            bars: Default::default(),
        }
    }
}

impl Drop for TermProgress {
    fn drop(&mut self) {
        for (bar, _) in &self.bars {
            bar.finish_and_clear();
        }
    }
}

impl ProgressListener for TermProgress {
    fn on_start(&mut self, name: &str) {
        for (bar, running) in &mut self.bars {
            if running.is_some() {
                continue;
            }

            bar.set_message(name.to_string());
            bar.reset();
            *running = Some(name.to_string());
            return;
        }

        let p = self.multi.add(ProgressBar::new_spinner());
        p.set_message(name.to_string());
        p.enable_steady_tick(Duration::from_millis(50));

        self.bars.push((p, Some(name.to_string())));
    }

    fn on_finish(&mut self, name: &str) {
        let (bar, running) = self
            .bars
            .iter_mut()
            .find(|(_, r)| r.as_ref() == Some(&name.to_string()))
            .expect("called on_finish without on_start");

        *running = None;
        bar.set_message("");
        bar.finish();
    }
}

#[derive(Default)]
struct ContinuousIntegrationProgress {
    total: usize,
    running: BTreeMap<String, Instant>,
    finished: BTreeMap<String, Duration>,
}

impl ContinuousIntegrationProgress {
    fn new(total: usize) -> Arc<Mutex<Self>> {
        eprintln!("Running {total} tasks");

        let progress = Arc::new(Mutex::new(ContinuousIntegrationProgress {
            total,
            running: Default::default(),
            finished: Default::default(),
        }));

        let weak = Arc::downgrade(&progress);
        std::thread::spawn(move || {
            while let Some(arc) = weak.upgrade() {
                arc.lock().unwrap().log_status();
                drop(arc);

                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        });

        progress
    }

    fn log_status(&self) {
        eprintln!(
            "Running {}, finished {} / {}",
            self.running.len(),
            self.finished.len(),
            self.total
        );
        for (name, started) in &self.running {
            eprintln!(
                "  {name}: {}",
                humantime::format_duration(started.elapsed())
            );
        }
    }
}

impl ProgressListener for Arc<Mutex<ContinuousIntegrationProgress>> {
    fn on_start(&mut self, name: &str) {
        eprintln!("Starting {name}");
        self.lock()
            .unwrap()
            .running
            .insert(name.to_string(), Instant::now());
    }

    fn on_finish(&mut self, name: &str) {
        let mut lock = self.lock().unwrap();

        let started_at = lock
            .running
            .remove(name)
            .expect("called on_finish without on_start");
        let took = started_at.elapsed();
        eprintln!("Finished {name} in {}", humantime::format_duration(took));

        lock.finished.insert(name.to_string(), took);
    }
}

impl Drop for ContinuousIntegrationProgress {
    fn drop(&mut self) {
        eprintln!("Runtime report:");

        let mut sorted_order = self.finished.iter().collect::<Vec<_>>();
        sorted_order.sort_by_key(|(_, d)| *d);

        for (name, dur) in sorted_order {
            eprintln!("  {}: {name}", humantime::format_duration(*dur));
        }
    }
}
