use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::commands::DependencyView;
use crate::error::{CliError, ExitStatus};

/// Appends the mutually exclusive `--downstream`, `--upstream`, and `--all` direction flags to a
/// subcommand. Combining more than one raises `EX_USAGE`; omitting all behaves as `--downstream`.
pub fn dependency_command(name: &'static str, about_kind: &'static str) -> Command {
    Command::new(name)
        .about(format!(
            "Render the dependency tree for the {about_kind} project"
        ))
        .arg(
            Arg::new("downstream")
                .long("downstream")
                .action(ArgAction::SetTrue)
                .help("Render the downstream dependency tree (default)."),
        )
        .arg(
            Arg::new("upstream")
                .long("upstream")
                .action(ArgAction::SetTrue)
                .help("Render the upstream dependency tree."),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Render both upstream and downstream dependency trees."),
        )
}

/// Parses the direction flags, enforcing mutual exclusivity.
pub fn parse_view(matches: &ArgMatches) -> Result<DependencyView, CliError> {
    let downstream = matches.get_flag("downstream");
    let upstream = matches.get_flag("upstream");
    let all = matches.get_flag("all");

    let selected = [downstream, upstream, all].iter().filter(|&&b| b).count();
    if selected > 1 {
        return Err(CliError::new(
            "use only one of --downstream, --upstream, or --all",
            ExitStatus::Usage,
        ));
    }

    Ok(if all {
        DependencyView::All
    } else if upstream {
        DependencyView::Upstream
    } else {
        DependencyView::Downstream
    })
}
