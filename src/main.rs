extern crate ini;
extern crate keepass;
extern crate newline_converter;

use ini::Ini;
use keepass::{Database, DatabaseKey};
use newline_converter::dos2unix;

use std::env;
use std::fs::File;
use std::process;
use std::io::{self, Write};

struct KeePassConfig {
    db_path: String,
    db_pass: String,
}

fn main() -> std::io::Result<()> {
    let stdout = io::stdout();
    let mut out_handle = stdout.lock();
    let stderr = io::stderr();
    let mut err_handle = stderr.lock();

    let args: Vec<_> = env::args_os().collect();

    // Handle flags
    if args.len() > 1 {
        let arg = args[1].to_str().unwrap();
        if arg == "-V" || arg == "--version" {
            out_handle.write(format!("{}\n", env!("CARGO_PKG_VERSION")).as_bytes()).unwrap();
            out_handle.flush().unwrap();
            process::exit(0);
        }
        if arg == "-h" || arg == "--help" {
            out_handle.write(get_help_text().as_bytes()).unwrap();
            out_handle.flush().unwrap();
            process::exit(0);
        }
    }

    if args.len() <= 1 {
        err_handle.write(b"no variable was provided").unwrap();
        err_handle.flush().unwrap();
        process::exit(1);
    }

    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(error_msg) => {
            err_handle.write(error_msg.as_bytes()).unwrap();
            err_handle.flush().unwrap();
            process::exit(1);
        }
    };

    let keepass_db_path = &config.db_path;
    let keepass_db_pass = &config.db_pass;

    let db_path = std::path::Path::new(keepass_db_path);
    let key = DatabaseKey::new().with_password(keepass_db_pass);
    let db = Database::open(&mut File::open(db_path)?, key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    let secret_path = args[1].to_str().unwrap();
    let secret_vec = secret_path.split("|").collect::<Vec<&str>>();

    let field;
    if secret_vec.len() == 2 {
        field = secret_vec[1];
    }
    else if secret_vec.len() == 1 {
        field = "Password";
    }
    else {
        err_handle.write(format!("{} is no valid secret path", secret_path).as_bytes()).unwrap();
        err_handle.flush().unwrap();
        process::exit(2);

    }
    let (group_path, entry_name) = secret_vec[0].rsplit_once('/').unwrap_or(("", secret_vec[0]));
    let path_segments: Vec<&str> = group_path.split('/').filter(|s| !s.is_empty()).collect();
    let entry = db.root.group_by_path(&path_segments).and_then(|g| g.entry_by_name(entry_name));
    if let Some(e) = entry {
        // Check if the field exists
        if let Some(field_value) = e.get(field) {
            out_handle.write(dos2unix(field_value).as_bytes()).unwrap();
            out_handle.flush().unwrap();
            process::exit(0);
        } else {
            err_handle.write(format!("{} could not be retrieved", secret_path).as_bytes()).unwrap();
            err_handle.flush().unwrap();
            process::exit(1);
        }
    }

    err_handle.write(format!("{} could not be retrieved", secret_path).as_bytes()).unwrap();
    err_handle.flush().unwrap();
    process::exit(1);
}

/// Generate help text explaining configuration and usage
fn get_help_text() -> String {
    format!(r#"summon-keepass {}

Summon provider for reading secrets from KeePass (.kdbx) database files.

USAGE:
    summon-keepass [OPTIONS] <SECRET_PATH>
    summon-keepass -h|--help
    summon-keepass -V|--version

OPTIONS:
    -h, --help       Display this help message
    -V, --version    Display version information

SECRET PATH FORMAT:
    [group/subgroup/]entry[|field]

    By default, the 'Password' field is returned. To retrieve a different
    field, append |field_name to the entry path.

EXAMPLES:
    summon-keepass "simple-entry"
        Returns the Password field from 'simple-entry'

    summon-keepass "aws/iam/user/robot"
        Returns the Password field from nested entry

    summon-keepass "account|UserName"
        Returns the UserName field from 'account'

    summon-keepass "aws/iam/user/robot|access_key_id"
        Returns the access_key_id field from nested entry

CONFIGURATION:

    Option 1: Environment Variables (Recommended for projects)
        export SUMMON_KEEPASS_DB_PATH=/path/to/database.kdbx
        export SUMMON_KEEPASS_DB_PASS="your database password"

    Option 2: Configuration File
        Create ~/.summon-keepass.ini with:

        [keepass_db]
        path=/path/to/database.kdbx
        pass=your database password

    Priority: Environment variables override configuration file.
    You can also mix sources (e.g., path from env, password from file).

EXIT CODES:
    0    Success
    1    Configuration error, entry not found, or field not found
    2    Invalid secret path format

For more information, visit:
    https://github.com/desolat/summon-keepass
"#, env!("CARGO_PKG_VERSION"))
}

/// Load configuration from environment variables and/or INI file
/// Priority: Environment variables > ~/.summon-keepass.ini
fn load_config() -> Result<KeePassConfig, String> {
    // Try environment variables first
    let env_path = env::var("SUMMON_KEEPASS_DB_PATH").ok();
    let env_pass = env::var("SUMMON_KEEPASS_DB_PASS").ok();

    // Try INI file as fallback
    let (ini_path, ini_pass) = load_ini_config();

    // Merge with priority (env vars override INI)
    let db_path = env_path.clone().or(ini_path.clone());
    let db_pass = env_pass.clone().or(ini_pass.clone());

    // Validate both are present
    match (db_path, db_pass) {
        (Some(path), Some(pass)) => Ok(KeePassConfig {
            db_path: path,
            db_pass: pass,
        }),
        _ => Err(build_config_error(&env_path, &env_pass, &ini_path, &ini_pass)),
    }
}

/// Load configuration from ~/.summon-keepass.ini file
/// Returns (Option<path>, Option<password>)
fn load_ini_config() -> (Option<String>, Option<String>) {
    // Get HOME directory (return None if not available)
    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => return (None, None),
    };

    let config_path = format!("{}/.summon-keepass.ini", home);

    // Try to load INI file (return None if fails)
    let config = match Ini::load_from_file(&config_path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };

    // Try to get section and values
    let section = config.section(Some("keepass_db"));
    match section {
        Some(s) => (
            s.get("path").map(|p| p.to_string()),
            s.get("pass").map(|p| p.to_string()),
        ),
        None => (None, None),
    }
}

/// Build a helpful error message showing what configuration sources were checked
fn build_config_error(
    env_path: &Option<String>,
    env_pass: &Option<String>,
    ini_path: &Option<String>,
    ini_pass: &Option<String>,
) -> String {
    let mut msg = String::from("Configuration error: Could not load KeePass database configuration.\n\n");

    msg.push_str("Checked sources:\n");

    // Environment variables
    msg.push_str("  Environment variables:\n");
    msg.push_str(&format!("    SUMMON_KEEPASS_DB_PATH: {}\n",
        if env_path.is_some() { "✓ Found" } else { "✗ Not set" }));
    msg.push_str(&format!("    SUMMON_KEEPASS_DB_PASS: {}\n",
        if env_pass.is_some() { "✓ Found" } else { "✗ Not set" }));

    // INI file
    msg.push_str("  Configuration file (~/.summon-keepass.ini):\n");
    if ini_path.is_some() || ini_pass.is_some() {
        msg.push_str(&format!("    path: {}\n",
            if ini_path.is_some() { "✓ Found" } else { "✗ Missing" }));
        msg.push_str(&format!("    pass: {}\n",
            if ini_pass.is_some() { "✓ Found" } else { "✗ Missing" }));
    } else {
        msg.push_str("    ✗ File not found or invalid format\n");
    }

    msg.push_str("\nRequired: Both database path and password must be configured.\n");
    msg.push_str("Set either:\n");
    msg.push_str("  - Environment variables: SUMMON_KEEPASS_DB_PATH and SUMMON_KEEPASS_DB_PASS\n");
    msg.push_str("  - Or create ~/.summon-keepass.ini with [keepass_db] section\n");

    msg
}
