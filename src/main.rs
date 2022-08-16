use colored::Colorize;
use fabricgen::*;

fn main() {
    println!("{}", "[ FabricGen ]".bold().bright_cyan().to_string());

    println!("Available versions:");
    for version in ["1.19", "1.18", "1.17", "1.16"] {
        if version == "1.19" {
            println!(
                "{} {} {}",
                "-".dimmed(),
                version.bright_cyan(),
                "(Latest)".dimmed(),
            );
        } else {
            println!("{}", format!("- {}", version).dimmed());
        }
    }

    let args = ProjectGenArgs::from_user_input();

    make_mod(&args).unwrap_or_else(|e| {
        eprintln!(
            "{}\n{}: {}",
            "An error occured!".bold().bright_red(),
            e.kind().to_string().red(),
            e.into_inner().unwrap_or("unknown".into())
        );
    })
}
