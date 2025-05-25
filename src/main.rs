use std::fs;
use std::env;
use std::io;
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;
use chrono::Local;
use config::{Config, ConfigBuilder, File, Environment};

use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{Pool, mssql::Mssql};
use tokio;
use std::process::Command;

fn read_config(config_path: &str) -> Result<Config, config::ConfigError> {
    let builder: ConfigBuilder<_> = Config::builder();

    let settings = builder
        .add_source(File::with_name(config_path))
        .add_source(Environment::with_prefix("APP"))
        .build()?;

    Ok(settings)
}


fn move_additional_files(source: &str, destination: &str,folder:&str) -> io::Result<()> {
    let src_path = Path::new(source);
    let dest_path = Path::new(destination).join(folder);

    // Create the "Email" folder if it doesn't exist
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path)?;
    }

    // Get the current date to match the file modified date

    // Regex pattern to match filenames with 10 digits followed by a hyphen
    let file_pattern = Regex::new(r"^\d{10}-").unwrap();

    // Walk through the source directory
    for entry in WalkDir::new(src_path).min_depth(1).max_depth(2).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let file_name_str = file_name.to_string_lossy();

            // Check if the file name matches the pattern
            if file_pattern.is_match(&file_name_str) {
                // Get the file's metadata to check the modified date
                if let Ok(metadata) = fs::metadata(path) {
                    if let Ok(modified_time) = metadata.modified() {
                        let current_system_time = SystemTime::now();
                        let modified_since_epoch = modified_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
                        let current_day_since_epoch = current_system_time.duration_since(UNIX_EPOCH).unwrap().as_secs() / 86400;
                        let modified_day_since_epoch = modified_since_epoch / 86400;

                        // Check if the file was modified on the current day
                        if current_day_since_epoch == modified_day_since_epoch {
                            let dest_file = dest_path.join(file_name);

                            // Use fs::copy instead of fs::rename
                            fs::copy(path, &dest_file)?;

                            println!("Moved file: {:?}", path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn move_files(source: &str, destination: &str) -> io::Result<()> {
    let src_path = Path::new(source);
    let dest_path = Path::new(destination);

    // Get the current date and format it as dd-mm-yyyy
    let current_date = Local::now().format("%d-%m-%Y").to_string();

    // Create the new folder in the destination with the current date as its name
    let dated_folder = dest_path.join(&current_date);
    if !dated_folder.exists() {
        fs::create_dir_all(&dated_folder)?;
    }

    // Regex pattern to match filenames with 10 digits followed by a hyphen
    let file_pattern = Regex::new(r"^\d{10}-").unwrap();

    // Walk through the source directory and its subdirectories
    for entry in WalkDir::new(src_path).min_depth(1).max_depth(2).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let file_name_str = file_name.to_string_lossy();

            // Check if the file name matches the pattern
            if file_pattern.is_match(&file_name_str) {
                let dest_file = dated_folder.join(file_name);  // Save file in the dated folder
                
                // Use fs::copy instead of fs::rename
                fs::copy(path, &dest_file)?;

                // Remove the original file after copying
                // fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}


async fn execute_stored_procedure(pool: &Pool<Mssql>, procedure_name: &str) -> Result<(), sqlx::Error> {
    // Execute the stored procedure
    sqlx::query(&format!("EXEC {}", procedure_name))
        .execute(pool)
        .await?;

    println!("Stored procedure executed successfully.");
    Ok(())
}

async fn check_table_for_data(pool: &Pool<Mssql>, table_name: &str) -> Result<bool, sqlx::Error> {
    // Run a SELECT query to check for data
    let row: Option<(i32,)> = sqlx::query_as(&format!("SELECT TOP 1 1 FROM {}", table_name))
        .fetch_optional(pool)
        .await?;

    if row.is_some() {
        println!("Data found in table {}.", table_name);
        Ok(true)
    } else {
        println!("No data found in table {}.", table_name);
        Ok(false)
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {

    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.toml"
    };

    let config = read_config(config_path).expect("Failed to load configuration");

    // Read source and destination paths from the config
    let base_source_folder = config.get_string("paths.source").expect("Failed to read source path");
    let destination_folder = config.get_string("paths.destination").expect("Failed to read destination path");

    let current_date = Local::now().format("%d-%m-%Y").to_string();
    let full_source_folder = format!("{}/{}", base_source_folder, current_date);

    // Read database URL and JAR file path from the config
    let database_url = config.get_string("database.url").expect("Failed to read database URL");
    let jar_file_path = config.get_string("paths.jar_file").expect("Failed to read JAR file path");
    let procedure_name = config.get_string("paths.procedure_name").expect("Failed to read procedure");

    // Set up the database connection pool
    let pool = Pool::<Mssql>::connect(&database_url).await?;

    // Executing the stored procedure from the config
    execute_stored_procedure(&pool, &procedure_name).await?;

    // Check if there is data in the table
    let table_name = "MasterTableABC"; // Replace with your actual table name
    let has_data = check_table_for_data(&pool, table_name).await?;

    if has_data {
        println!("Proceeding with further operations...");

        // Running the JAR file
        let output = Command::new("java")
            .arg("-jar")
            .arg(&jar_file_path)
            .output()
            .expect("Failed to execute JAR file");

        // Checking the output or handle any errors
        if output.status.success() {
            println!("JAR file executed successfully.");
            // Moving files from the source folder to the destination folder
            match move_files(&full_source_folder, &destination_folder) {
                Ok(()) =>{
                    //Moving Email file
                    match move_additional_files("D/OndemandGeneration/Mail", &destination_folder,"Email") {
                        Ok(()) => println!("Mail files moved successfully!"),
                        Err(e) => eprintln!("Error moving Mail files: {}", e),
                    }

                    // Moving SMS files
                    match move_additional_files("D/OndemandGeneration/SMS", &destination_folder,"SMS") {
                        Ok(()) => println!("SMS files moved successfully!"),
                        Err(e) => eprintln!("Error moving SMS files: {}", e),
                    }
                },
                Err(e) => eprintln!("Error moving files: {}", e),
            }
        } else {
            eprintln!("Failed to run JAR file. Error: {:?}", output.stderr);
        }
    } else {
        println!("No data found, program will exit.");
    }

    Ok(())
}
