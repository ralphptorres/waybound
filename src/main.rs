use clap::Parser;
use std::collections::HashSet;
use wayland_client::{Connection, EventQueue};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

mod wayland;
use crate::wayland::{BoundaryPlacement, BoundaryRule, WaylandState};

#[derive(Clone, Debug)]
struct RuleArg {
    boundary: String,
    command: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'r', long = "rule", value_name = "BOUNDARY=COMMAND", num_args = 1..)]
    rule: Vec<String>,

    #[arg(long)]
    debug: bool,
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

fn placement_priority(placement: &BoundaryPlacement) -> u8 {
    if placement.width > 0 && placement.height > 0 {
        1
    } else {
        0
    }
}

fn build_rules(args: &Args) -> Result<Vec<BoundaryRule>, Box<dyn std::error::Error>> {
    if args.rule.is_empty() {
        return Err("at least one --rule bound=command is required".into());
    }

    let mut seen_bounds = HashSet::new();
    let mut rules = Vec::new();

    for rule in &args.rule {
        let parsed = parse_rule(rule).ok_or_else(|| format!("invalid rule '{}', expected bound=command", rule))?;
        let placement = parse_boundary(&parsed.boundary);

        if !seen_bounds.insert(placement.name.clone()) {
            if args.debug {
                println!("[debug] skipping duplicate bound: {}", placement.name);
            }
            continue;
        }

        rules.push(BoundaryRule {
            placement,
            command: parsed.command,
        });
    }

    rules.sort_by_key(|rule| placement_priority(&rule.placement));
    Ok(rules)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let conn = Connection::connect_to_env()?;
    let mut event_queue: EventQueue<WaylandState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let rules = build_rules(&args)?;

    let mut state = WaylandState::new(rules, args.debug);

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state)?;

    if state.is_ready() {
        if args.debug {
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
