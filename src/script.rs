// used to determine if the file is a valid script or not
// TODO(zph) conditionally check for executable here
use std::path::Path;
use crate::commands::complete::is_valid_command;
use std::path::PathBuf;

use super::types::CommandType;
use std::{
    fs::File, io::{self, prelude::*, BufReader, Read}, process::{Command, Stdio}
};

pub fn is_tome_script(filename: &str) -> bool {
    !filename.starts_with('.')
}

/// Any executable script
/// can be added to be executed, but
/// It's possible to add metadata
/// to the script via comments as well.
pub struct Script {
    pub help_string: String,
    /// the string that should be used for
    /// usage information
    /// the path the script is located at.
    pub path: String,
    /// determines if the script should
    /// be sourced or not.
    pub should_source: bool, /// determines if the script should
    /// have completion invoked or not.
    pub should_complete: bool,
    /// the string that should be printed
    /// when help is requested.
    pub summary_string: String,
    /// syntax
    pub language: &'static Language,
    pub display: bool,
    pub filetype: &'static str,
}

pub enum LanguageType {
    Python,
    Shell,
    Ruby,
    Javascript,
    Typescript,
    Text,
}

pub struct Language {
    pub extension: &'static str,
    pub name: &'static str,
    pub language: LanguageType,
    pub comment_character: &'static str,
}

const LANGUAGES: [Language; 6] = [
    Language{extension: "bash", name: "bash", language: LanguageType::Shell, comment_character: "#"},
    Language{extension: "sh", name: "sh", language: LanguageType::Shell, comment_character: "#"},
    Language{extension: "py", name: "python", language: LanguageType::Python, comment_character: "#"},
    Language{extension: "ts", name: "typescript", language: LanguageType::Typescript, comment_character: "//"},
    Language{extension: "js", name: "javascript", language: LanguageType::Javascript, comment_character: "//"},
    Language{extension: "rb", name: "ruby", language: LanguageType::Ruby, comment_character: "#"},
];

const TEXT_LANGUAGE: Language = Language{extension: "", name: "text", language: LanguageType::Text, comment_character: "#"};

impl Script {
    fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }

    pub fn get_language_from_first_line(path: String) -> Option<&'static Language> {
        if let Ok(lines) = Script::read_lines(path) {
                for line in lines.flatten().take(1) {
                    return Script::shabang_to_language(line);
                }
            }
        return None
    }

    pub fn extension_to_language(ext: String) -> Option<&'static Language> {
        return LANGUAGES.iter().find(|&l| ext.ends_with(&l.extension))
    }

    pub fn shabang_to_language(shabang: String) -> Option<&'static Language> {
        return LANGUAGES.iter().find(|&l| shabang.contains(&l.name))
    }

    pub fn load(path: &str) -> io::Result<Script> {
        let file = Box::new(File::open(path)?) as Box<dyn Read>;
        Ok(Script::load_from_buffer(path.to_owned(), file))
    }
    pub fn load_from_buffer(path: String, body: Box<dyn Read>) -> Script {
        let mut buffer = BufReader::new(body);
        let mut should_complete = false;
        let mut should_source = false;
        // TODO(zph) Help String is never used, update help command to pull that info
        let mut help_string = String::new();
        let mut summary_string = String::new();
        let mut line = String::new();
        let mut consuming_help = false;
        let display = is_valid_command(PathBuf::from(path.clone()));

        let language = match Script::extension_to_language(path.clone()) {
            Some(x) => { x }
            None => {
                match Script::get_language_from_first_line(path.clone()) {
                    Some(x) => { x }
                    None =>  { &TEXT_LANGUAGE }
                }
            }
        };
        let comment = language.comment_character;

        loop {
            line.clear();
            match buffer.read_line(&mut line) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                }
                Err(_) => break,
            }
            if line.starts_with(&comment) {
                let mut trimmed_line = line.trim_start_matches(&comment);
                trimmed_line = trimmed_line.trim_start();
                if consuming_help {
                    if trimmed_line.starts_with("END HELP") {
                        consuming_help = false;
                    } else {
                        // omit first two characters since they are
                        // signifying continued help.
                        help_string.push_str(trimmed_line);
                    }
                } else if trimmed_line.starts_with("COMPLETE") {
                    should_complete = true;
                } else if trimmed_line.starts_with("SOURCE") {
                    should_source = true;
                } else if trimmed_line.starts_with("START HELP") {
                    consuming_help = true;
                } else if trimmed_line.starts_with("SUMMARY") {
                    // 9 = prefix, -1 strips newtrimmed_line
                    summary_string.push_str(&trimmed_line.trim_start_matches(":")[9..(trimmed_line.len() - 1)]);
                } else if !line.starts_with("#!") {
                    // if a shebang is encountered, we skip.
                    // as it can indicate the command to run the script with.
                    // metadata lines must be consecutive.
                    break;
                }

            }
        }

        let filetype = if should_source {
            "s"
        } else {
            "x"
        };

        Script {
            help_string,
            path,
            should_complete,
            should_source,
            summary_string,
            language,
            display,
            filetype,
        }
    }

    // return the appropriate string that should be executed within the
    // function.
    pub fn get_execution_body(
        &self,
        command_type: CommandType,
        shell: &str,
        args: &[&String],
    ) -> Result<String, String> {
        match command_type {
            CommandType::Completion => {
                if !self.should_complete {
                    return Ok(String::new());
                }
                // in the completion case, we need to execute the script itself.
                // There's a possible optimization here
                // if we just inherit parent file descriptors.
                let mut command = match self.should_source {
                    true => Command::new(shell),
                    false => Command::new(self.path.clone()),
                };
                if self.should_source {
                    command.arg(self.path.clone());
                }
                command.arg("--complete");
                let command_output = command.args(args).stdout(Stdio::piped()).output();
                match command_output {
                    Ok(output) => match String::from_utf8(output.stdout) {
                        Err(error) => Err(format!(
                            "unable to parse completion results as a utf8 string: {}",
                            error
                        )),
                        Ok(result) => Ok(result),
                    },
                    // TODO: it's hard to get output from a completion call.
                    // possible to print to stderr?
                    Err(result) => Err(format!("completion called failed: {}", result)),
                }
            }
            CommandType::Execute => {
                let command_string = if self.should_source {
                    // when sourcing, just return the full body.
                    let mut command = vec![String::from("."), self.path.clone()];
                    for arg in args.iter() {
                        command.push((**arg).clone());
                    }
                    // in the case of sourcing, at least one variable needs
                    // to be specified, or else arguments will be inherited
                    // from the parent process.
                    if command.len() == 2 {
                        command.push(String::from(""));
                    }
                    command
                } else {
                    let mut command = vec![self.path.clone()];
                    for arg in args.iter() {
                        command.push((**arg).clone());
                    }
                    command
                };
                // after figuring out the command, all resolved values
                // should be quoted, to ensure that the shell does not
                // interpret character sequences.
                let mut escaped_command_string = vec![];
                for mut arg in command_string {
                    arg = arg.replace('\'', "\\'");
                    arg.insert(0, '\'');
                    arg.push('\'');
                    escaped_command_string.push(arg);
                }
                Ok(escaped_command_string.join(" "))
            }
        }
    }
}
