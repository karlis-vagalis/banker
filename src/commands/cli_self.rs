use crate::cli::SelfFormat;
use crate::error::AppError;
use clap::CommandFactory;

pub async fn run(format: SelfFormat) -> Result<(), AppError> {
    let mut cmd = crate::cli::Cli::command();
    match format {
        SelfFormat::Tree => print_command_tree(&mut cmd),
        SelfFormat::List => print_command_list(&mut cmd),
        SelfFormat::Systemd { .. } => unreachable!("handled by systemd::run"),
    }
    Ok(())
}

fn format_args(cmd: &mut clap::Command) -> String {
    let mut arg_strings = Vec::new();
    for arg in cmd.get_arguments() {
        if arg.get_id() == "help" || arg.get_id() == "version" {
            continue;
        }

        let is_required = arg.is_required_set();

        let mut formatted = if arg.get_action().takes_values() {
            if let Some(short) = arg.get_short() {
                format!("-{} <{}>", short, arg.get_id())
            } else if let Some(long) = arg.get_long() {
                format!("--{} <{}>", long, arg.get_id())
            } else {
                let base = format!("<{}>", arg.get_id());
                let is_multiple = arg
                    .get_num_args()
                    .map(|range| range.max_values() > 1)
                    .unwrap_or(false);
                if is_multiple {
                    format!("{}...", base)
                } else {
                    base
                }
            }
        } else if let Some(long) = arg.get_long() {
            format!("--{}", long)
        } else {
            format!("-{}", arg.get_short().unwrap())
        };

        if !is_required {
            formatted = format!("[{}]", formatted);
        }

        arg_strings.push(formatted);
    }

    if arg_strings.is_empty() {
        String::new()
    } else {
        format!(" {}", arg_strings.join(" "))
    }
}

fn print_commands(
    cmd: &mut clap::Command,
    format: &SelfFormat,
    prefix: String,
    is_last: bool,
    path: Vec<String>,
) {
    let name = cmd.get_name().to_string();
    let args_str = format_args(cmd);

    let about = cmd
        .get_about()
        .or_else(|| cmd.get_long_about())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let mut subcommands: Vec<clap::Command> = cmd
        .get_subcommands()
        .filter(|c| !c.is_hide_set() && c.get_name() != "help")
        .cloned()
        .collect();
    subcommands.sort_by(|a, b| a.get_name().cmp(b.get_name()));

    match format {
        SelfFormat::Systemd { .. } => unreachable!("handled by systemd::run"),
        SelfFormat::Tree => {
            let aliases: Vec<String> = cmd.get_visible_aliases().map(|s| s.to_string()).collect();
            let alias_str = if !aliases.is_empty() {
                format!(" (alias: {})", aliases.join(", "))
            } else {
                String::new()
            };

            if prefix.is_empty() {
                println!("{}{}", name, args_str);
            } else {
                let connector = if is_last { "└── " } else { "├── " };
                if !about.is_empty() {
                    println!(
                        "{}{}{}{}{} - {}",
                        prefix,
                        connector,
                        name,
                        alias_str,
                        args_str,
                        about.trim()
                    );
                } else {
                    println!("{}{}{}{}{}", prefix, connector, name, alias_str, args_str);
                }
            }

            let next_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            let len = subcommands.len();
            for (i, child) in subcommands.iter_mut().enumerate() {
                print_commands(child, format, next_prefix.clone(), i == len - 1, vec![]);
            }
        }

        SelfFormat::List => {
            let mut current_path = path;
            current_path.push(name);

            let is_leaf = subcommands.is_empty();
            let is_runnable = is_leaf || !cmd.is_subcommand_required_set();

            if is_runnable {
                let full_path = current_path.join(" ");
                let aliases: Vec<String> =
                    cmd.get_visible_aliases().map(|s| s.to_string()).collect();
                let alias_str = if !aliases.is_empty() {
                    format!(" (alias: {})", aliases.join(", "))
                } else {
                    String::new()
                };
                if !about.is_empty() {
                    println!("{}{}{} - {}", full_path, alias_str, args_str, about.trim());
                } else {
                    println!("{}{}{}", full_path, alias_str, args_str);
                }
            }

            for child in subcommands.iter_mut() {
                print_commands(child, format, String::new(), false, current_path.clone());
            }
        }
    }
}

fn print_command_tree(cmd: &mut clap::Command) {
    print_commands(cmd, &SelfFormat::Tree, String::new(), true, vec![]);
}

fn print_command_list(cmd: &mut clap::Command) {
    print_commands(cmd, &SelfFormat::List, String::new(), true, vec![]);
}
