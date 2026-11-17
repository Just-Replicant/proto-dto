// SPDX-License-Identifier: AGPL-3.0-or-later

//! Prints the mirror and check files for messages of a descriptor set,
//! without writing anything. Handy to preview a `build.rs` change:
//!
//! ```sh
//! protoc --include_source_info -o /tmp/rf_grpc.bin -I proto proto/explorer/folder.proto
//! cargo run -p proto-dto-build --example render -- /tmp/rf_grpc.bin rf_grpc rf_proto FolderDTO
//! ```

use std::process::ExitCode;

use proto_dto_build::Mirror;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [descriptor, package, import, messages @ ..] = args.as_slice() else {
        eprintln!("usage: render <descriptor.bin> <package> <import> <Message>...");
        return ExitCode::FAILURE;
    };
    if messages.is_empty() {
        eprintln!("usage: render <descriptor.bin> <package> <import> <Message>...");
        return ExitCode::FAILURE;
    }

    let rendered = Mirror::new(descriptor)
        .package(package.as_str())
        .import(import.as_str())
        .file("mirror.rs", messages.iter().map(String::as_str))
        .check_file("check.rs")
        .render();
    match rendered {
        Ok(files) => {
            for file in files {
                println!("// ----- {} -----", file.path.display());
                print!("{}", file.contents);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
