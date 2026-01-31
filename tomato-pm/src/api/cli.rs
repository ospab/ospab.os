pub fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("Usage: tomato-pm <command> [package]".to_string());
    }

    match args[1].as_str() {
        "install" => {
            if args.len() < 3 {
                Err("install requires a package name".to_string())
            } else {
                Ok(Command::Install(args[2].clone()))
            }
        }
        "remove" | "uninstall" => {
            if args.len() < 3 {
                Err("remove requires a package name".to_string())
            } else {
                Ok(Command::Remove(args[2].clone()))
            }
        }
        "list" => Ok(Command::List),
        "search" => {
            if args.len() < 3 {
                Err("search requires a query".to_string())
            } else {
                Ok(Command::Search(args[2].clone()))
            }
        }
        _ => Err(format!("Unknown command: {}", args[1])),
    }
}

#[derive(Debug)]
pub enum Command {
    Install(String),
    Remove(String),
    List,
    Search(String),
}