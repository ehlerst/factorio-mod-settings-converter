use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use factorio_mod_settings_converter::ModSettings;

#[derive(Parser, Debug)]
#[command(author, version, about = "Converts Factorio mod-settings.dat files to/from JSON and YAML", long_about = "Automatically detects format based on file extension (.dat, .json, .yaml, .yml).")]
struct Args {
    /// Path to the input file (.dat, .json, .yaml, or .yml)
    input: PathBuf,
    /// Path to the output file (defaults to changing .dat to .json, or vice-versa)
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = &args.input;
    let input_name = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid input filename")?;

    let mod_settings: ModSettings;

    if input_name.ends_with(".dat") {
        eprintln!("Reading DAT file '{:?}'...", input_path);
        let mut file = BufReader::new(File::open(input_path)?);
        mod_settings = ModSettings::load(&mut file)?;
    } else if input_name.ends_with(".json") {
        eprintln!("Reading JSON file '{:?}'...", input_path);
        let file = File::open(input_path)?;
        mod_settings = serde_json::from_reader(file)?;
    } else if input_name.ends_with(".yaml") || input_name.ends_with(".yml") {
        eprintln!("Reading YAML file '{:?}'...", input_path);
        let file = File::open(input_path)?;
        mod_settings = serde_yml::from_reader(file)?;
    } else {
        return Err(anyhow!(
            "Input filename '{:?}' does not end with .dat, .json, or .yaml/.yml.",
            input_path
        ));
    }

    let output_path = args.output.unwrap_or_else(|| {
        if input_name.ends_with(".dat") {
            input_path.with_extension("json")
        } else {
            input_path.with_extension("dat")
        }
    });

    let output_name = output_path
        .to_str()
        .context("Invalid output path")?;

    if output_name.ends_with(".dat") {
        eprintln!("Writing DAT file '{:?}'...", output_path);
        let mut file = BufWriter::new(File::create(&output_path)?);
        mod_settings.save(&mut file)?;
    } else if output_name.ends_with(".json") {
        eprintln!("Writing JSON file '{:?}'...", output_path);
        let file = File::create(&output_path)?;
        serde_json::to_writer_pretty(file, &mod_settings)?;
    } else if output_name.ends_with(".yaml") || output_name.ends_with(".yml") {
        eprintln!("Writing YAML file '{:?}'...", output_path);
        let file = File::create(&output_path)?;
        serde_yml::to_writer(file, &mod_settings)?;
    } else {
        return Err(anyhow!(
            "Output filename '{:?}' does not end with .dat, .json, or .yaml/.yml.",
            output_path
        ));
    }

    Ok(())
}
