extern crate clap;

#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::process::*;
use clap::*;
use std::fs::File;
use std::io::Read;
use std::io::prelude::*;
use std::io;

fn main() {
    let matches = App::new("Backup tool")
        .version("0.1")
        .author("Nicolas Silva <nical@fastmail.com>")
        .arg(Arg::with_name("VERBOSE")
            .short("v")
            .long("verbose")
            .help("Verbose output.")
            .takes_value(false)
        )
        .arg(Arg::with_name("DEBUG")
            .short("d")
            .long("debug")
            .help("Print the commands but do not run them.")
            .takes_value(false)
        )
        .subcommand(
            params(
                SubCommand::with_name("push")
                    .about("Push files without deleting anything.")
            )
        )
        .subcommand(
            params(
                SubCommand::with_name("sync")
                    .about("Synchronize the directories (deleting extranous files on the target).")
            )
        )
        .subcommand(SubCommand::with_name("init")
            .about("Set up a .backup.toml config file.")
            .arg(Arg::with_name("SRC")
                .help("Source directory.")
                .value_name("SRC")
                .takes_value(true)
            )
        )
        .get_matches();

    fn params<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.arg(Arg::with_name("SRC")
            .help("Source directory.")
            .value_name("SRC")
            .takes_value(true)
        )
        .arg(Arg::with_name("TARGET")
            .short("o")
            .long("output")
            .help("Target directory")
            .value_name("TARGET")
            .takes_value(true)
        )
        .arg(Arg::with_name("USER")
            .short("u")
            .long("user")
            .help("User name on the server")
            .value_name("USER")
            .takes_value(true)
        )
        .arg(Arg::with_name("SERVER")
            .short("s")
            .long("server")
            .help("Server address.")
            .value_name("SERVER")
            .takes_value(true)
        )
        .arg(Arg::with_name("PORT")
            .short("p")
            .long("port")
            .value_name("PORT")
            .takes_value(true)
        )
    }

    let dbg = matches.is_present("DEBUG");
    let verbose = matches.is_present("VERBOSE") || dbg;
    let mut src_dir = ".".to_string();
    let mut user = "rsync".to_string();
    let mut server = "".to_string();
    let mut port = 22;
    let mut target_dir = "~/backups/default/".to_string();
    let mut cmd = None;
    let mut extra_args = String::new();
    let mut excludes = Vec::new();

    if let Some(init_cmd) = matches.subcommand_matches("init") {
        if let Some(v) = init_cmd.value_of("SRC") {
            src_dir = v.to_string();
        }

        init(&src_dir).unwrap();
        return;
    }

    if let Some(mut config_file) = File::open(format!("{}/.backup.toml", src_dir)).ok() {
        if verbose {
            println!(" ** Found config file at {}/.backup.toml", src_dir);
        }
        let mut buf = Vec::new();
        config_file.read_to_end(&mut buf).expect("Failed to read config file.");
        let config: Config = toml::from_slice(&buf[..]).unwrap();

        user = config.user.unwrap_or(user);
        server = config.server.unwrap_or(server);
        target_dir = config.target_dir.unwrap_or(target_dir);
        port = config.port.unwrap_or(port);
        if let Some(command) = config.default {
            if command == "push" {
                cmd = Some(Cmd::Push);
            }
            if command == "sync" {
                cmd = Some(Cmd::Sync);
            }
        }

        if let Some(e) = config.exclude {
            excludes = e;
        }
    }

    if let Some(push_cmd) = matches.subcommand_matches("push") {
        cmd = Some(Cmd::Push);
        get_subcommand_params(push_cmd, &mut user, &mut server, &mut target_dir, &mut src_dir, &mut port);
    } else if let Some(sync_cmd) = matches.subcommand_matches("sync") {
        cmd = Some(Cmd::Sync);
        get_subcommand_params(sync_cmd, &mut user, &mut server, &mut target_dir, &mut src_dir, &mut port);
    } else if let Some(sync_cmd) = matches.subcommand_matches("fetch") {
        cmd = Some(Cmd::Fetch);
        get_subcommand_params(sync_cmd, &mut user, &mut server, &mut target_dir, &mut src_dir, &mut port);
    }

    if cmd.is_none() {
        println!("No command to run.");
    }

    let cmd = cmd.unwrap();

    if cmd == Cmd::Fetch {
        unimplemented!();
    }

    if server == "" {
        panic!("Must specify a server.");
    }

    if user == "" {
        panic!("Must specify a user.");
    }

    if verbose {
        extra_args += " -v"
    }

    let addr = &format!("{}@{}:{}",
        user,
        server,
        target_dir,
    );

    let ssh_cmd = format!("ssh -p {}", port);

    let mut args = vec![
        "-Parz",
        "-e", &ssh_cmd,
        &src_dir,
        addr,
    ];

    for e in &excludes {
        args.push("--exclude");
        args.push(e);
    }

    if cmd == Cmd::Sync {
        args.push("--delete");
    }

    if verbose {
        println!(" ** command: push");
        args.push("-v");
    }

    let mut command = Command::new("rsync");
    command.args(args);

    if verbose {
        println!("{:?}", command);
    }

    if !dbg {
        let status = command.status().expect("Failed to execute the command.");
        assert!(status.success());
    }
}

fn get_subcommand_params(
    cmd: &ArgMatches,
    user: &mut String,
    server: &mut String,
    target_dir: &mut String,
    src_dir: &mut String,
    port: &mut u32,
) {
    if let Some(v) = cmd.value_of("USER") {
        *user = v.to_string();
    }
    if let Some(v) = cmd.value_of("SERVER") {
        *server = v.to_string();
    }
    if let Some(v) = cmd.value_of("TARGET") {
        *target_dir = v.to_string();
    }

    if let Some(v) = cmd.value_of("SRC") {
        *src_dir = v.to_string();
    }

    if let Some(v) = cmd.value_of("PORT") {
        *port = v.parse().unwrap();
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Cmd {
    Push,
    Sync,
    Fetch,
}

#[derive(Deserialize)]
struct Config {
    server: Option<String>,
    user: Option<String>,
    port: Option<u32>,
    target_dir: Option<String>,
    default: Option<String>,
    exclude: Option<Vec<String>>,
}

fn init(src_dir: &str) -> io::Result<()> {
    let path = format!("{}/.backup.toml", src_dir);
    fn read_input(label: &str) -> String {
        let stdin = io::stdin();
        let mut input = stdin.lock().lines();
        print!("{}", label);
        io::stdout().flush().unwrap();
        input.next().unwrap().unwrap()
    }

    let address = read_input("Server address: ");
    let mut port = read_input("port [22]: ");
    if port == "" {
        port = "22".to_string();
    }
    let target_dir = read_input("Target directory on the server: ");
    let user = read_input("User on the server: ");
    let mut default = None;
    while match read_input("Use a command by default? (push/sync/fetch): ").as_str() {
        "push" => {
            default = Some("push".to_string());
            false
        }
        "sync" => {
            default = Some("sync".to_string());
            false
        }
        "" => {
            false
        }
        _ => {
            println!("Unknown command.");
            true
        }
    } {}

    let mut excludes = Vec::new();
    while match read_input("Exclude pattern? : ").as_str() {
        "" => { false }
        pat => {
            excludes.push(pat.to_string());
            true
        }
    } {}

    if let Some(mut config_file) = File::create(path.clone()).ok() {
        writeln!(config_file, "server = \"{}\"", address)?;
        writeln!(config_file, "port = \"{}\"", port)?;
        writeln!(config_file, "user = \"{}\"", user)?;
        writeln!(config_file, "target_dir = \"{}\"", target_dir)?;
        if let Some(default) = default {
            writeln!(config_file, "default = \"{}\"", default)?;
        }

        writeln!(config_file, "exclude = [")?;
        for (i, pat) in excludes.iter().enumerate() {
            if i == excludes.len() - 1 {
                writeln!(config_file, "    \"{}\"", pat)?;
            } else {
                writeln!(config_file, "    \"{}\",", pat)?;
            }
        }
        writeln!(config_file, "]")?;

        println!("Created configuration file at {}", path);
    }

    Ok(())
}