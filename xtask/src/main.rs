use std::env;
use std::error::Error;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    let command = args[1].as_str();
    let result = match command {
        "generate" => handle_generate(),
        "create-issues" => handle_removed_create_issues(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("xtask: unknown command '{}'", other);
            print_help();
            std::process::exit(1);
        }
    };

    if let Err(err) = result {
        eprintln!("xtask error: {}", err);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("Bunny workspace automation tasks.");
    println!();
    println!("Usage: cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  generate        Regenerate Rust and TypeScript DTOs from graphics schema");
    println!("  create-issues   Removed; GitHub Issues are the canonical backlog");
    println!("  help            Show this help info");
}

fn handle_generate() -> Result<(), Box<dyn Error>> {
    println!("Generating graphics DTOs...");
    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "bunny-wesley",
            "--",
            "schemas/bunny/v0/graphics.graphql",
            "--rust",
            "crates/bunny-contract/src/generated/graphics.rs",
            "--typescript",
            "generated/typescript/bunny-graphics.ts",
            "--manifest",
            "generated/bunny-graphics.manifest.json",
        ])
        .status()?;

    if !status.success() {
        return Err("cargo run -p bunny-wesley failed".into());
    }

    println!("Generation completed successfully.");
    Ok(())
}

fn handle_removed_create_issues() -> Result<(), Box<dyn Error>> {
    Err(concat!(
        "`create-issues` has been removed: GitHub Issues are the canonical backlog, ",
        "so xtask must not generate or mutate issues from local roadmap data."
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_issues_command_is_fail_closed() {
        let err = handle_removed_create_issues()
            .expect_err("create-issues must not mutate GitHub from local roadmap data")
            .to_string();

        assert!(err.contains("GitHub Issues are the canonical backlog"));
        assert!(err.contains("must not generate or mutate issues"));
    }
}
