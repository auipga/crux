mod data;
mod format;
mod logic;
mod parser;

use std::fs::File;

use anyhow::{bail, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use rustdoc_types::Crate;
use tokio::task::spawn_blocking;

use crate::args::CodegenArgs;

pub async fn codegen(args: &CodegenArgs) -> Result<()> {
    let mut cmd = MetadataCommand::new();
    let package_graph = PackageGraph::from_command(&mut cmd)?;

    let Ok(lib) = package_graph.workspace().member_by_path(&args.lib) else {
        bail!("Could not find workspace package with path {}", args.lib)
    };

    let json_path = rustdoc_json::Builder::default()
        .toolchain("nightly")
        .document_private_items(true)
        .manifest_path(lib.manifest_path())
        .build()?;

    let crate_: Crate = spawn_blocking(move || -> Result<Crate> {
        let file = File::open(json_path)?;
        let crate_ = serde_json::from_reader(file)?;
        Ok(crate_)
    })
    .await??;

    let data = data::Data::new(crate_);

    let parsed = parser::parse(&data);

    let registry = logic::run(parsed);
    println!("{:#?}", registry);

    Ok(())
}
