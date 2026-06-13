mod render;

use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Config {
    schema: PathBuf,
    rust: PathBuf,
    typescript: PathBuf,
    manifest: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bunny-wesley: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = parse_args(env::args().skip(1))?;
    let schema_source = fs::read_to_string(&config.schema)?;
    let schema_ir = wesley_core::lower_schema_sdl(&schema_source)?;

    let mut hasher = Sha256::new();
    hasher.update(schema_source.as_bytes());
    let schema_hash = format!("{:x}", hasher.finalize());

    write_file(
        &config.rust,
        &render::render_rust(&schema_ir, &schema_hash, &config.schema)?,
    )?;
    write_file(
        &config.typescript,
        &render::render_typescript(&schema_ir, &schema_hash, &config.schema)?,
    )?;
    write_file(&config.manifest, &render_manifest(&config, &schema_hash))?;

    Ok(())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Config, Box<dyn Error>> {
    let mut schema = None;
    let mut rust = None;
    let mut typescript = None;
    let mut manifest = None;
    let mut pending_flag = None::<String>;

    for arg in args {
        if let Some(flag) = pending_flag.take() {
            match flag.as_str() {
                "--rust" => rust = Some(PathBuf::from(arg)),
                "--typescript" => typescript = Some(PathBuf::from(arg)),
                "--manifest" => manifest = Some(PathBuf::from(arg)),
                _ => return Err(format!("unknown flag {flag}").into()),
            }
            continue;
        }

        match arg.as_str() {
            "--rust" | "--typescript" | "--manifest" => pending_flag = Some(arg),
            "--help" | "-h" => return Err(usage().into()),
            value if value.starts_with('-') => {
                return Err(format!("unknown flag {value}\n{}", usage()).into());
            }
            value => {
                if schema.replace(PathBuf::from(value)).is_some() {
                    return Err(format!("multiple schema paths supplied\n{}", usage()).into());
                }
            }
        }
    }

    if let Some(flag) = pending_flag {
        return Err(format!("missing value for {flag}").into());
    }

    Ok(Config {
        schema: schema.ok_or_else(usage)?,
        rust: rust.ok_or_else(usage)?,
        typescript: typescript.ok_or_else(usage)?,
        manifest: manifest.ok_or_else(usage)?,
    })
}

fn usage() -> String {
    "usage: bunny-wesley <schema.graphql> --rust <path> --typescript <path> --manifest <path>"
        .to_string()
}

fn render_manifest(config: &Config, schema_sha256: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"generator\": \"{}\",\n",
            "  \"wesleyCore\": \"{}\",\n",
            "  \"schema\": \"{}\",\n",
            "  \"schemaSha256\": \"{}\",\n",
            "  \"outputs\": [\n",
            "    \"{}\",\n",
            "    \"{}\"\n",
            "  ]\n",
            "}}\n"
        ),
        render::GENERATOR_ID,
        render::WESLEY_CORE_VERSION,
        json_escape(&config.schema.display().to_string()),
        schema_sha256,
        json_escape(&config.rust.display().to_string()),
        json_escape(&config.typescript.display().to_string())
    )
}

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
