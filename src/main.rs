use clap::Parser;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use wayland_client::{Connection, EventQueue};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

mod wayland;
use crate::wayland::{BoundaryPlacement, BoundaryRule, WaylandState};

#[derive(Clone, Debug)]
struct RuleArg {
    boundary: String,
    command: String,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    debug: bool,

    #[serde(default)]
    boundaries: Vec<ConfigBoundary>,
}

#[derive(Debug, Deserialize)]
struct ConfigBoundary {
    boundary: String,
    command: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'c',
        long,
        value_name = "config",
        help = "path [default: $XDG_CONFIG_HOME/waybound/waybound.toml]"
    )]
    config: Option<PathBuf>,

    #[arg(
        short = 'r',
        long = "rule",
        value_name = "boundary=command",
        num_args = 1..,
        help = "add a boundary rule, like top-left=... or right=..."
    )]
    rule: Vec<String>,

    #[arg(long, help = "show debug output and make boundary overlays visible")]
    debug: bool,
}

fn default_config_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")?;
    Some(PathBuf::from(base).join("waybound").join("waybound.toml"))
}

fn parse_boundary(boundary: &str) -> BoundaryPlacement {
    use zwlr_layer_surface_v1::Anchor;
    let lower = boundary.to_lowercase();
    match lower.as_str() {
        "top-left" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Top | Anchor::Left,
            width: 10,
            height: 10,
        },
        "top-right" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Top | Anchor::Right,
            width: 10,
            height: 10,
        },
        "bottom-left" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Left,
            width: 10,
            height: 10,
        },
        "bottom-right" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Right,
            width: 10,
            height: 10,
        },
        "top" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            width: 0,
            height: 10,
        },
        "bottom" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            width: 0,
            height: 10,
        },
        "left" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Left | Anchor::Top | Anchor::Bottom,
            width: 10,
            height: 0,
        },
        "right" => BoundaryPlacement {
            name: lower,
            anchor: Anchor::Right | Anchor::Top | Anchor::Bottom,
            width: 10,
            height: 0,
        },
        _ => {
            eprintln!("unknown boundary '{}', defaulting to top-left", boundary);
            BoundaryPlacement {
                name: "top-left".to_string(),
                anchor: Anchor::Top | Anchor::Left,
                width: 10,
                height: 10,
            }
        }
    }
}

fn parse_rule(rule: &str) -> Option<RuleArg> {
    let (placement, command) = rule.split_once('=')?;
    let placement = placement.trim();
    let command = command.trim();

    if placement.is_empty() || command.is_empty() {
        return None;
    }

    Some(RuleArg {
        boundary: placement.to_string(),
        command: command.to_string(),
    })
}

fn load_config(path: &PathBuf) -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn insert_rule(
    rules: &mut Vec<BoundaryRule>,
    seen_bounds: &mut HashSet<String>,
    rule: BoundaryRule,
    allow_override: bool,
) {
    if seen_bounds.contains(&rule.placement.name) {
        if allow_override {
            rules.retain(|existing| existing.placement.name != rule.placement.name);
            rules.push(rule);
        }
        return;
    }

    seen_bounds.insert(rule.placement.name.clone());
    rules.push(rule);
}

fn add_config_rules(
    config: ConfigFile,
    rules: &mut Vec<BoundaryRule>,
    seen_bounds: &mut HashSet<String>,
) {
    for entry in config.boundaries {
        insert_rule(
            rules,
            seen_bounds,
            BoundaryRule {
                placement: parse_boundary(&entry.boundary),
                command: entry.command,
            },
            false,
        );
    }
}

fn placement_priority(placement: &BoundaryPlacement) -> u8 {
    if placement.width > 0 && placement.height > 0 {
        1
    } else {
        0
    }
}

fn build_rules(
    args: &Args,
) -> Result<(Vec<BoundaryRule>, bool, Option<PathBuf>), Box<dyn std::error::Error>> {
    let mut rules = Vec::new();
    let mut seen_bounds = HashSet::new();
    let mut debug = args.debug;
    let mut loaded_config = None;

    if let Some(path) = &args.config {
        let config = load_config(path)?;
        debug |= config.debug;
        loaded_config = Some(path.clone());
        add_config_rules(config, &mut rules, &mut seen_bounds);
    } else if let Some(path) = default_config_path() {
        if path.exists() {
            let config = load_config(&path)?;
            debug |= config.debug;
            loaded_config = Some(path.clone());
            add_config_rules(config, &mut rules, &mut seen_bounds);
        }
    }

    for rule in &args.rule {
        let parsed = parse_rule(rule).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("error: invalid rule '{}', expected bound=command", rule),
            )
        })?;
        let placement = parse_boundary(&parsed.boundary);

        let rule = BoundaryRule {
            placement,
            command: parsed.command,
        };

        if seen_bounds.contains(&rule.placement.name) {
            rules.retain(|existing| existing.placement.name != rule.placement.name);
            rules.push(rule);
        } else {
            seen_bounds.insert(rule.placement.name.clone());
            rules.push(rule);
        }
    }

    if rules.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "error: provide --config, --rule, or a default config file",
        )
        .into());
    }

    rules.sort_by_key(|rule| placement_priority(&rule.placement));
    Ok((rules, debug, loaded_config))
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let conn = Connection::connect_to_env()?;
    let mut event_queue: EventQueue<WaylandState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let (rules, debug, loaded_config) = build_rules(&args)?;

    let mut state = WaylandState::new(rules, debug);

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state)?;

    if state.is_ready() {
        if debug {
            if let Some(path) = loaded_config {
                println!("[debug] config loaded: {}", path.display());
            }
            for rule in &state.rules {
                println!("[debug] boundary configured: {}", rule.placement.name);
            }
        }
        state.create_surfaces(&qh)?;
    } else {
        eprintln!("error: failed to bind required wayland globals");
    }

    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
