use rayon::prelude::*;
use std::io::Write;
use std::{path::Path, sync::RwLock};
use walkdir::WalkDir;

pub mod api_parser;
pub use crate::api_parser::*;

/// Parse a given file and return the resulting data
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ApiDef> {
    let api_gen = ApiParser::parse_file(path)?;
    // TODO: Second pass
    Ok(api_gen)
}

/// Given a path load all the files and parse them.
pub fn parse_files<P: AsRef<Path>>(path: P, print_process: bool) -> Result<Vec<ApiDef>> {
    let wd = WalkDir::new(path);

    let files = wd
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().metadata().unwrap().is_file())
        .collect::<Vec<_>>();

    let api_defs = RwLock::new(Vec::with_capacity(files.len()));

    // Pass 1: Parse all the files

    files.par_iter().for_each(|f| {
        if print_process {
            println!("Parsing file {:?}", f.path());
        }

        let api_def = ApiParser::parse_file(f.path()).unwrap();

        // Insert the api_def for later usage
        {
            let mut data = api_defs.write().unwrap();
            data.push(api_def);
        }
    });

    let mut data = api_defs.into_inner().unwrap();

    //ApiParser::second_pass(&mut data);
    data.sort_by(|a, b| a.filename.cmp(&b.filename));

    Ok(data)
}

/// Hepler function to write C style comments
pub fn write_c_commments<W: Write>(f: &mut W, comments: &Vec<String>, indent: usize) -> Result<()> {
    for c in comments {
        writeln!(f, "{:indent$}// {}", "", c, indent = indent)?;
    }

    Ok(())
}
