use crate::directory::is_tomeignored;

use super::super::{
    directory, script,
    types::{CommandType, TargetType},
};
use super::builtins::BUILTIN_COMMANDS;
use std::{fs, io, path::PathBuf};

pub enum ScriptType {
    Source,
    Executable,
    Other,
}

pub fn script_type_to_string(script_type: ScriptType) -> &'static str {
    match script_type {
        ScriptType::Executable => { &"executable" }
        ScriptType::Source => { &"source" }
        ScriptType::Other => { &"other" }
    }
}

pub fn command_type(path: PathBuf) -> ScriptType {
    let content = fs::read_to_string(path).unwrap_or(String::from(""));
    let lines = content.split_once("\n").unwrap_or((&"", &""));
    if lines.0.starts_with("#!") {
        return ScriptType::Executable;
    } else if lines.0.contains("SOURCE") {
        return ScriptType::Source;
    }
    return ScriptType::Other;
}

pub fn is_valid_command(path: PathBuf) -> bool {
    let cmd = command_type(path);
    return match cmd {
        ScriptType::Executable => { true }
        ScriptType::Source => { true }
        ScriptType::Other => { false }
    }
}

pub fn complete(
    command_directory_path: &str,
    shell: &str,
    args: &[String],
) -> Result<String, String> {
    // TODO: refactor to share common logic with execute
    // determine if a file or a directory was passed,
    // recursing down arguments until we've exhausted arguments
    // that match a directory or file.
    let mut target = PathBuf::from(&command_directory_path);
    let mut target_type = TargetType::Directory;
    let mut args_peekable = args.iter().peekable();
    // handle if the first command is a builtin
    if let Some(subcommand) = args_peekable.peek() {
        if BUILTIN_COMMANDS.get(*subcommand).is_some() {
            return Ok(String::new());
        }
    }
    while let Some(arg) = args_peekable.peek() {
        target.push(arg);
        if target.is_file() {
            if is_valid_command(target.clone()) {
                target_type = TargetType::File;
                args_peekable.next();
            } else {
                target.pop();
            }
            break;
        } else if target.is_dir() {
            target_type = TargetType::Directory;
            args_peekable.next();
        } else {
            // the current argument does not match
            // a directory or a file, so we've landed
            // on the strictest match.
            target.pop();
            break;
        }
    }
    let remaining_args: Vec<_> = args_peekable.collect();

    return match target_type {
        TargetType::Directory => {
            let paths_raw: io::Result<_> = fs::read_dir(target.to_str().unwrap());
            let mut paths: Vec<_> = match paths_raw {
                Err(_a) => return Err("Invalid argument to completion".to_string()),
                Ok(a) => a,
            }
            .filter_map(|r| match r {
                Ok(path_buf) => {
                    let path = path_buf.path();
                    if path.is_dir() && !directory::is_tome_script_directory(&path) {
                        return None;
                    }
                    let tomeignored = is_tomeignored(&command_directory_path.to_string(), path.clone());
                    if path.is_file() && (!script::is_tome_script(
                            path_buf.file_name().to_str().unwrap_or_default(),
                        ) || tomeignored)
                    {
                        return None;
                    }
                    Some(path.file_name().unwrap().to_str().unwrap_or("").to_owned())
                }
                Err(_) => None,
            })
            .collect();
            // if this is the root directory, add the builtin commands
            if target.to_str().unwrap() == command_directory_path {
                for command in BUILTIN_COMMANDS.keys() {
                    paths.push(command.to_owned());
                }
            }
            paths.sort_by_key(|f| f.to_owned());
            Ok(paths.join(" "))
        }
        TargetType::File => match script::Script::load(target.to_str().unwrap_or_default()) {
            Ok(script) => {
                script.get_execution_body(CommandType::Completion, shell, &remaining_args)
            }
            Err(error) => return Err(format!("IOError loading file: {:?}", error)),
        },
    };
}
