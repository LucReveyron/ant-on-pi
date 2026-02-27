use std::fmt;

#[derive(Debug, PartialEq)]
pub enum LoopControl {
    Continue,
    Break,
}

#[derive(Debug, PartialEq)]
enum Command {
    Shutdown,
    Help,
    Current,
    List,
    // Add more commands here as needed
}

// Implement Display for Command
impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Shutdown => write!(f, "shutdown"),
            Command::Current => write!(f, "current"),
            Command::Help => write!(f, "help"),
            Command::List => write!(f, "list"),
            // Add more commands here as needed
        }
    }
}

fn get_help_message() -> String {
    format!(
        "Available commands:\n\
        - \\shutdown: Shuts down the system.\n\
        - \\current: Displays the current Job.\n\
        - \\help: Shows this help message.\n\
        - \\list: Shows all Jobs in memory."
        // Add more commands here as needed
    )
}

fn identify_commands(input: &str) -> Vec<Command> {
    let mut commands = Vec::new();
    let words: Vec<&str> = input.split_whitespace().collect();

    for word in words {
        match word {
            "\\shutdown" => commands.push(Command::Shutdown),
            "\\current" => commands.push(Command::Current),
            "\\help" => commands.push(Command::Help),
            "\\list" => commands.push(Command::List),
            // Add more matches for new commands
            _ => continue,
        }
    }

    commands
}

pub fn resolve_commands(input: &str) -> (LoopControl, Vec<String>) {
    let mut shutdown_encountered = false;
    let mut responses = Vec::new();

    let commands = identify_commands(input);

    for cmd in commands {

        match cmd {
            Command::Shutdown => {
                shutdown_encountered = true;
                responses.push("Executing shutdown...".to_string());
            }
            Command::Current => responses.push("Executing current...".to_string()), // TODO: 02/27/26 add search in scheduler.redb for current Job
            Command::Help => responses.push(get_help_message()),
            // TODO: 02/27/26 Command::List => return_list_jobs() return all Jobs stored in scheduler.redb
            // Add more commands here as needed
            _ => responses.push(format!("Not implemented command: {}", cmd.to_string())),
        }
    }

    if shutdown_encountered {
        (LoopControl::Break, responses)
    } else {
        (LoopControl::Continue, responses)
    }
}