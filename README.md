# Rust Stored Procedure Executor and File Mover

This Rust application performs the following tasks in sequence:

1. Reads configuration from `config.toml`
2. Connects to an MS SQL Server database
3. Executes a stored procedure
4. Checks a table for available data
5. Runs a JAR file if data exists
6. Moves generated files to destination folders (organized by date and type)

## 🔧 Configuration

Edit the `config.toml` file to set the following values:

```toml
[database]
url = "sqlserver://user:password@localhost/database"

[paths]
source = "D:/OndemandGeneration"
destination = "D:/Destination"
jar_file = "D:/Utility/processor.jar"
procedure_name = "your_stored_procedure_name"
```

## ▶️ Running the Project

Use the following command to run the project:

```bash
cargo run --release
```

Or specify a custom config file:

```bash
cargo run --release -- path/to/your_config.toml
```

## 🗃️ Output

- Files are moved into date-stamped folders.
- Email and SMS files are copied into respective subfolders inside the destination.

## 🛠️ Dependencies

- `sqlx`
- `tokio`
- `chrono`
- `regex`
- `walkdir`
- `config`

Ensure you have the proper SQL Server credentials and file permissions.

## 📦 Build

```bash
cargo build --release
```
