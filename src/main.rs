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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Cmd {
    Push,
    Sync,
    Fetch,
}

#[derive(Deserialize)]
struct Config {
    server: Option<String>,
    port: Option<u32>,
    user: Option<String>,
    target_dir: Option<String>,
    default: Option<String>,
    update_permissions: Option<bool>,
    update_dir_times: Option<bool>,
    exclude: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct GlobalConfig {
    server: Option<String>,
    port: Option<u32>,
    user: Option<String>,
    exclude: Option<Vec<String>>,
    update_permissions: Option<bool>,
    update_dir_times: Option<bool>,
    // List of directories to back up when --all is used.
    list: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct Params {
    cmd: Option<Cmd>,
    ip: String,
    port: u32,
    user: String,
    src_dir: String,
    target_dir: String,
    exclude: Vec<String>,
    all: bool,
    update_dir_times: bool,
    update_permissions: bool,
    verbose: bool,
    dbg: bool,
}

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
        .arg(Arg::with_name("ALL")
            .long("all")
            .help("Backup all folders listed in the global config file (~/.backup.toml).")
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
        .subcommand(
            SubCommand::with_name("when")
            .about("When is the last time folder was backed up.")
            .arg(Arg::with_name("WRITE")
                .short("w")
                .long("write")
                .help("Write the current date into the date file")
                .takes_value(false)
            )
        )
        .subcommand(
            SubCommand::with_name("info")
            .about("Print some information about the backed up directory.")
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

    let mut params = Params {
        cmd: None,
        ip: "".to_string(),
        port: 22,
        user: "".to_string(),
        src_dir: ".".to_string(),
        target_dir: "~/backups/default/".to_string(),
        exclude: Vec::new(),
        dbg: matches.is_present("DEBUG"),
        verbose: matches.is_present("VERBOSE"),
        all: matches.is_present("ALL"),
        update_dir_times: false,
        update_permissions: false,
    };
    params.verbose |= params.dbg;


    if let Some(init_cmd) = matches.subcommand_matches("init") {
        if let Some(v) = init_cmd.value_of("SRC") {
            params.src_dir = v.to_string();
        }

        init(&params.src_dir).unwrap();
        return;
    }

    let global = read_global_config_file(&mut params);
    get_cmd(&mut params, &matches);

    if params.all {
        if let Some(GlobalConfig { list: Some(directories), .. }) = global {
            for dir in &directories {
                if params.verbose {
                    println!(" **");
                    println!(" ** {}", dir);
                }

                let mut params = params.clone();
                params.src_dir = dir.to_string();

                if let Some(_) = matches.subcommand_matches("info") {
                    info_command(&params);
                } else if let Some(when_cmd) = matches.subcommand_matches("when") {
                    when_command(&params, &when_cmd);
                } else {
                    read_config_file(&mut params);
                    execute(&params);
                }
            }
        }

        return;
    }

    if let Some(_) = matches.subcommand_matches("info") {
        info_command(&params);
    } else if let Some(when_cmd) = matches.subcommand_matches("when") {
        when_command(&params, &when_cmd);
    } else {
        read_config_file(&mut params);
        execute(&params);
    }
}

fn execute(params: &Params) {
    let mut extra_args = String::new();

    if params.cmd.is_none() {
        println!(" No command to run.");
        return;
    }

    let cmd = params.cmd.unwrap();

    if cmd == Cmd::Fetch {
        unimplemented!();
    }

    if params.ip == "" {
        panic!("Must specify a server.");
    }

    if params.user == "" {
        panic!("Must specify a user.");
    }

    if !params.dbg {
        write_date_file(&params.src_dir, params.verbose);
    }

    if params.verbose {
        extra_args += " -v"
    }

    let addr = &format!("{}@{}:{}",
        params.user,
        params.ip,
        params.target_dir,
    );

    let ssh_cmd = format!("ssh -p {}", params.port);

    let mut args = vec![
        "-Parz",
        "-e", &ssh_cmd,
        &params.src_dir,
        addr,
    ];

    if !params.update_dir_times {
        args.push("--omit-dir-times");
    }
    if !params.update_permissions {
        args.push("--no-perms")
    }

    for e in &params.exclude {
        args.push("--exclude");
        args.push(e);
    }

    if cmd == Cmd::Sync {
        args.push("--delete");
    }

    if params.verbose {
        println!(" ** command: push");
        args.push("-v");
    }

    let mut command = Command::new("rsync");
    command.args(args);

    if params.verbose {
        println!(" ** {:?}", command);
    }

    if !params.dbg {
        let status = command.status().expect("Failed to execute the command.");
        assert!(status.success());
    }
}

fn info_command(params: &Params) {
    if let Some(date) = read_date_file(&params.src_dir, params.verbose) {
        println!(" - folder: {:?}, last backup: {}", params.src_dir, date.to_string());
    } else {
        println!(" - folder: {:?}", params.src_dir);
    }
}

fn when_command(params: &Params, when_cmd: &ArgMatches) {
    if when_cmd.is_present("WRITE") {
        write_date_file(&params.src_dir, params.verbose);
    } else {
        if let Some(date) = read_date_file(&params.src_dir, params.verbose) {
            println!("{}", date.to_string());
        }
    }
}

fn get_cmd(params: &mut Params, matches: &clap::ArgMatches) {
    if let Some(push_cmd) = matches.subcommand_matches("push") {
        params.cmd = Some(Cmd::Push);
        get_subcommand_params(push_cmd, params);
    } else if let Some(sync_cmd) = matches.subcommand_matches("sync") {
        params.cmd = Some(Cmd::Sync);
        get_subcommand_params(sync_cmd, params);
    } else if let Some(sync_cmd) = matches.subcommand_matches("fetch") {
        params.cmd = Some(Cmd::Fetch);
        get_subcommand_params(sync_cmd, params);
    }
}


fn read_config_file(params: &mut Params) {
    let path = format!("{}/.backup.toml", params.src_dir);
    let mut config_file = match File::open(&path) {
        Ok(file) => file,
        Err(..) => {
            if params.verbose {
                println!(" ** No local config file ({})", path);
            }
            return;
        }
    };

    if params.verbose {
        println!(" ** Found local config file at {}/.backup.toml", params.src_dir);
    }

    let mut buf = Vec::new();
    config_file.read_to_end(&mut buf).expect("Failed to read config file.");
    let config: Config = toml::from_slice(&buf[..]).unwrap();

    if let Some(ip) = config.server {
        params.ip = ip;
    }
    if let Some(port) = config.port {
        params.port = port;
    }
    if let Some(user) = config.user {
        params.user = user;
    }
    if let Some(dir) = config.target_dir {
        params.target_dir = dir;
    }

    if let Some(update) = config.update_permissions {
        params.update_permissions = update;
    }

    if let Some(update) = config.update_dir_times {
        params.update_dir_times = update;
    }

    if params.cmd.is_none() {
        if let Some(command) = config.default {
            if command == "push" {
                params.cmd = Some(Cmd::Push);
            }
            if command == "sync" {
                params.cmd = Some(Cmd::Sync);
            }
        }
    }

    if let Some(e) = config.exclude {
        params.exclude.extend_from_slice(&e);
    }
}

fn read_global_config_file(params: &mut Params) -> Option<GlobalConfig> {
    let mut path = dirs::home_dir()?;
    path.push(".backup.toml");

    if params.verbose {
        println!(" ** Looking for global config file at: {:?}", path);
    }

    let mut config_file = File::open(path).ok()?;

    if params.verbose {
        println!(" ** Found global config file");
    }
    let mut buf = Vec::new();
    config_file.read_to_end(&mut buf).expect("Failed to read global config file.");
    let global: GlobalConfig = toml::from_slice(&buf[..]).unwrap();

    if let Some(ref ip) = global.server {
        params.ip = ip.clone();
    }
    if let Some(ref port) = global.port {
        params.port = port.clone();
    }
    if let Some(ref user) = global.user {
        params.user = user.clone();
    }

    if let Some(update) = global.update_permissions {
        params.update_permissions = update;
    }

    if let Some(update) = global.update_dir_times {
        params.update_dir_times = update;
    }

    if let Some(ref e) = global.exclude {
        params.exclude.extend_from_slice(e);
    }

    return Some(global);
}

fn get_subcommand_params(
    cmd: &ArgMatches,
    params: &mut Params,
) {
    if let Some(v) = cmd.value_of("USER") {
        params.user = v.to_string();
    }
    if let Some(v) = cmd.value_of("SERVER") {
        params.ip = v.to_string();
    }
    if let Some(v) = cmd.value_of("TARGET") {
        params.target_dir = v.to_string();
    }

    if let Some(v) = cmd.value_of("PORT") {
        params.port = v.parse().unwrap();
    }

    if let Some(v) = cmd.value_of("SRC") {
        assert!(!params.all, "Can't specify a source directory when --all is used.");
        params.src_dir = v.to_string();
    }
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
    let port = read_input("port: ");
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

    let mut exclude = Vec::new();
    while match read_input("Exclude pattern? : ").as_str() {
        "" => { false }
        pat => {
            exclude.push(pat.to_string());
            true
        }
    } {}

    if let Some(mut config_file) = File::create(path.clone()).ok() {
        if !address.is_empty() {
            writeln!(config_file, "server = \"{}\"", address)?;
        }
        if !port.is_empty() {
            writeln!(config_file, "port = {}", port)?;
        }
        if !user.is_empty() {
            writeln!(config_file, "user = \"{}\"", user)?;
        }
        writeln!(config_file, "target_dir = \"{}\"", target_dir)?;
        if let Some(default) = default {
            writeln!(config_file, "default = \"{}\"", default)?;
        }

        if !exclude.is_empty() {
            writeln!(config_file, "exclude = [")?;
            for (i, pat) in exclude.iter().enumerate() {
                if i == exclude.len() - 1 {
                    writeln!(config_file, "    \"{}\"", pat)?;
                } else {
                    writeln!(config_file, "    \"{}\",", pat)?;
                }
            }
        }
        writeln!(config_file, "]")?;

        println!("Created configuration file at {}", path);
    }

    Ok(())
}

type DateTime = chrono::DateTime<chrono::Local>;

fn date_path(src_dir: &str) -> String {
    format!("{}/.backup_time.txt", src_dir)
}

fn read_date_file(src_dir: &str, verbose: bool) -> Option<DateTime> {
    let path = date_path(src_dir);
    let date_str = match std::fs::read_to_string(&path) {
        Ok(f) => f,
        Err(e) => {
            if verbose {
                println!(" ** No date file: {:?}", e);
            }
            return None;
        }
    };

    if verbose {
        println!(" ** Found date file {}", path);
        println!(" ** {}", date_str);
    }

    let date = match date_str.parse::<DateTime>() {
        Ok(date) => date,
        Err(e) => {
            println!(" ** Failed to parse date {}", e);
            return None;
        }
    };

    Some(date)
}

fn write_date_file(src_dir: &str, verbose: bool) {
    let path = date_path(src_dir);
    if let Some(mut date_file) = File::create(&path).ok() {
        let date_str = chrono::Local::now().to_string();
        writeln!(date_file, "{}", date_str).unwrap();
        if verbose {
            println!(" ** Updated date file: {}", date_str);
        }
    }
}
