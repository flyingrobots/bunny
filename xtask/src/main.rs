//! Bunny workspace automation tasks.

mod code_dojo;
mod git_helpers;
mod repo_respect;
mod topic_docs;

use std::error::Error;
use std::process::Command;

fn main() {
    if let Err(err) = run() {
        eprintln!("xtask error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        std::process::exit(1);
    };

    match command.as_str() {
        "generate" => handle_generate(),
        "create-issues" => handle_removed_create_issues(),
        "code-dojo" => code_dojo::handle_full(args),
        "code-dojo-pre-commit" => code_dojo::handle_pre_commit(),
        "code-dojo-rust" => code_dojo::handle_rust(args),
        "code-dojo-determinism" => code_dojo::handle_determinism(args),
        "code-dojo-commit-msg" => code_dojo::handle_commit_msg(args),
        "repo-respect" => repo_respect::handle(args),
        "topic-docs" => topic_docs::handle(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("xtask: unknown command '{other}'");
            print_help();
            std::process::exit(1);
        }
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
    println!("  code-dojo       Run the full Code Dojo quality gate");
    println!("  code-dojo-pre-commit");
    println!("                  Run the local pre-commit Code Dojo gate");
    println!("  code-dojo-rust  Run Rust AST repository policy checks");
    println!("  code-dojo-determinism");
    println!("                  Check deterministic golden-vector receipts");
    println!("  code-dojo-commit-msg <path>");
    println!("                  Check a commit message file");
    println!("  repo-respect   Create or validate repo-respect receipts");
    println!("  topic-docs      Check topic test-plan requirement metadata");
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
        let err = match handle_removed_create_issues() {
            Ok(()) => String::new(),
            Err(err) => err.to_string(),
        };

        assert!(err.contains("GitHub Issues are the canonical backlog"));
        assert!(err.contains("must not generate or mutate issues"));
    }
}
