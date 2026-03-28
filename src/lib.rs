mod codegen;
mod parser;

use ocaml_interop::{OCaml, OCamlList, OCamlRuntime, ToOCaml};
use std::path::PathBuf;

#[ocaml_interop::export]
fn rust_process_files(
    cr: &mut OCamlRuntime,
    filenames: OCaml<OCamlList<String>>,
    prefix: OCaml<String>,
    output_dir: OCaml<String>,
    output_names: OCaml<String>,
) -> OCaml<String> {
    use crate::codegen::{generate, SelectedFunction};
    use crate::parser::parse_b_file;

    let files: Vec<String> = filenames.to_rust();
    let prefix: String = prefix.to_rust();
    let output_stem: String = output_names.to_rust();
    let output_dir = PathBuf::from(output_dir.to_rust::<String>());

    let result: String = (|| {
        let mut selections: Vec<SelectedFunction> = Vec::new();

        for path in &files {
            let src = match std::fs::read_to_string(path) {
                Ok(contents) => contents,
                Err(e) => return format!("Error: could not read file '{}': {}", path, e),
            };

            for sig in parse_b_file(&src) {
                let export_name = format!("{}{}", prefix, sig.name);
                selections.push(SelectedFunction { sig, export_name });
            }
        }

        if selections.is_empty() {
            return "Error: no valid function signatures found in the provided files.".to_string();
        }

        let header_name = format!("{}.h", output_stem);
        let files_out = generate(&selections, &header_name);

        let h_path = output_dir.join(&header_name);
        let c_path = output_dir.join(format!("{}.c", output_stem));

        if let Err(e) = std::fs::write(&h_path, &files_out.header) {
            return format!("Error: could not write header file '{:?}': {}", h_path, e);
        }

        if let Err(e) = std::fs::write(&c_path, &files_out.source) {
            return format!("Error: could not write source file '{:?}': {}", c_path, e);
        }

        format!(
            "Generated {} function(s) → {:?} and {:?}",
            selections.len(),
            h_path,
            c_path
        )
    })();

    result.to_ocaml(cr)
}