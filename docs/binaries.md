# Orca Binaries Documentation

## Table of Contents

*   [`orca-api`](#orca-api)
    *   [How to Run](#how-to-run)
    *   [Configuration (`api.json`)](#configuration-apijson)
    *   [Endpoints](#endpoints)
        *   [`POST /dispatch`](#post-dispatch)
*   [`orca-client`](#orca-client)
    *   [How to Run](#how-to-run-1)
    *   [Configuration (`client.json`)](#configuration-clientjson)
    *   [Functionality](#functionality)
*   [`orca-dispatch`](#orca-dispatch)
    *   [How to Run](#how-to-run-2)
    *   [Arguments](#arguments)
*   [`orca-registration`](#orca-registration)
    *   [How to Run](#how-to-run-3)
    *   [Configuration (`registration.json`)](#configuration-registrationjson)

This document provides detailed information on how to use each individual binary in the Orca project.

## `orca-api`

The `orca-api` binary provides a RESTful API for interacting with the Orca system, primarily for dispatching commands to clients.

### How to Run

To run the `orca-api` server, execute the following command from the project root:

```bash
cargo run --bin orca-api
```

### Configuration (`api.json`)

The `orca-api` server is configured via the `api.json` file located in the project root. An example `api.json`:

```json
{
  "listen_address": "127.0.0.1:8082",
  "dispatch_binary_path": "target/debug/orca-dispatch" // Path to the orca-dispatch executable
}
```

*   `listen_address`: The IP address and port on which the API server will listen for incoming requests.
*   `dispatch_binary_path`: The absolute or relative path to the `orca-dispatch` executable. This is used by the API server to spawn `orca-dispatch` as a subprocess.

### Endpoints

#### `POST /dispatch`

This endpoint is used to dispatch a command to a client, optionally including files.

*   **Method:** `POST`
*   **URL:** `/dispatch`
*   **Headers:** `Content-Type: application/json`

**Request Body:**

```json
{
    "command": "<command_to_execute>",
    "client": "<client_identifier>",
    "files": "<comma_separated_file_paths>" (optional)
}
```

*   `command` (string, required): The command string to be executed on the target client.
*   `client` (string, required): The identifier of the client to target. This can be a UUID, IP address, hostname, or MAC address.
*   `files` (string, optional): A comma-separated list of file paths (relative to where `orca-api` is run) to be sent to the client. These files will be placed in the client's workspace directory.

**Response Body:**

```json
{
    "stdout": "<standard_output_from_command>",
    "stderr": "<standard_error_from_command>",
    "success": <boolean>
}
```

*   `stdout` (string): The standard output captured from the executed command.
*   `stderr` (string): The standard error captured from the executed command.
*   `success` (boolean): `true` if the command executed successfully (exit code 0), `false` otherwise.

## `orca-client`

The `orca-client` binary registers itself with the `orca-registration` server and listens for commands from `orca-dispatch` (or `orca-api` indirectly).

### How to Run

To run the `orca-client`, execute the following command from the project root:

```bash
cargo run --bin orca-client
```

### Configuration (`client.json`)

The `orca-client` is configured via the `client.json` file located in the project root. An example `client.json`:

```json
{
  "registration_server": "127.0.0.1:8081",
  "listen_port": 8080,
  "log_file_path": "client.log",
  "log_level": "info",
  "workspace_dir": "./client_workspace" // Optional: default is orca-workspace next to executable
}
```

*   `registration_server`: The IP address and port of the `orca-registration` server.
*   `listen_port`: The port on which the client will listen for commands from `orca-dispatch`.
*   `log_file_path`: The path to the client's log file.
*   `log_level`: The minimum log level to record (e.g., "info", "debug", "error").
*   `workspace_dir` (optional): The directory where dispatched files will be saved and commands will be executed. If not specified, a directory named `orca-workspace` will be created next to the `orca-client` executable.

### Functionality

*   **Registration:** On startup, the client sends its UUID (or generates one if not present), hostname, IP, and MAC address to the registration server.
*   **Command Execution:** Listens for incoming commands. When a command is received, it creates/uses the `workspace_dir`, saves any accompanying files, executes the command within that directory, and sends the output back to the dispatcher. The workspace directory is automatically cleaned up after command execution.

## `orca-dispatch`

The `orca-dispatch` binary is a command-line tool used to send commands and files directly to an `orca-client`.

### How to Run

To run `orca-dispatch`, execute the following command from the project root:

```bash
cargo run --bin orca-dispatch -- <arguments>
```

### Arguments

*   `--command <COMMAND>` / `-c <COMMAND>` (required): The command string to be executed on the client.
*   `--client <CLIENT_IDENTIFIER>` / `-i <CLIENT_IDENTIFIER>` (required): The identifier of the client to target (UUID, IP, hostname, or MAC address).
*   `--files <FILE_PATHS>` / `-f <FILE_PATHS>` (optional): A comma-separated list of local file paths to send to the client. These files will be placed in the client's workspace directory.

**Example:**

```bash
cargo run --bin orca-dispatch -- --command "./my_script.sh" --client "192.168.1.100" --files "./script.sh,./config.txt"
```

## `orca-registration`

The `orca-registration` binary is the server responsible for managing client registrations and storing client information in a database.

### How to Run

To run the `orca-registration` server, execute the following command from the project root:

```bash
cargo run --bin orca-registration
```

### Configuration (`registration.json`)

The `orca-registration` server is configured via the `registration.json` file located in the project root. An example `registration.json`:

```json
{
  "database_url": "mysql://user:password@host:port/database",
  "listen_address": "127.0.0.1:8081"
}
```

*   `database_url`: The connection string for the MySQL database where client information will be stored.
*   `listen_address`: The IP address and port on which the registration server will listen for client registrations.