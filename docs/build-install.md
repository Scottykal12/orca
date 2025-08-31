# Building and Installing Orca

This document provides comprehensive instructions on how to build, install, and set up the Orca project components for both development and deployment.

## Table of Contents

*   [Prerequisites](#prerequisites)
*   [Database Setup](#database-setup)
*   [Building the Project](#building-the-project)
    *   [Building Debug Versions](#building-debug-versions)
    *   [Building Release Versions](#building-release-versions)
*   [Installation](#installation)
    *   [Windows Installation](#windows-installation)
    *   [Linux Installation](#linux-installation)
*   [Configuration Files](#configuration-files)
*   [Running the Project](#running-the-project)
    *   [Running from Source (Development)](#running-from-source-development)
    *   [Running Installed Binaries](#running-installed-binaries)

## Prerequisites

Before you can build and run Orca, you need to have the following installed:

*   **Rust:** The Rust programming language and its package manager, Cargo. You can install Rust by following the instructions on the [official Rust website](https://www.rust-lang.org/tools/install).
*   **MySQL Server:** A running MySQL server instance. Orca uses MySQL for client registration and event logging. You can download MySQL from the [official MySQL website](https://dev.mysql.com/downloads/mysql/).
*   **MySQL Client:** The `mysql` command-line client. This is often installed with the MySQL server, but ensure it's in your system's PATH.
*   **PowerShell (Windows):** Required for using the `manage-db.ps1` script.

## Database Setup

Orca requires a MySQL database to store client information and event logs. You can easily set up the necessary database and tables using the provided PowerShell script.

1.  **Ensure MySQL Server is Running:** Make sure your MySQL server is active and accessible (default: `localhost:3306`).
2.  **Run the Database Creation Script:**
    Navigate to the project root directory in PowerShell and execute:
    ```powershell
    . .\manage-db.ps1
    Create-Database
    ```
    This will create the `orca_db` database with `clients` and `events` tables. Refer to the [Database Management Script](manage-db.md) documentation for more details and customization options.

## Building the Project

Navigate to the root directory of the Orca project in your terminal.

### Building Debug Versions

Debug builds are useful for development and testing as they include debugging symbols and are compiled faster.

*   **Building All Binaries:**
    ```bash
    cargo build
    ```
    Executables will be located in `target/debug/`.

*   **Building Specific Binaries:**
    ```bash
    cargo build --bin orca-api
    cargo build --bin orca-client
    cargo build --bin orca-dispatch
    cargo build --bin orca-registration
    ```
    Specific executables will be in `target/debug/`.

### Building Release Versions

Release builds are optimized for performance and have smaller file sizes, making them suitable for deployment.

*   **Building All Binaries:**
    ```bash
    cargo build --release
    ```
    Executables will be located in `target/release/`.

*   **Building Specific Binaries:**
    ```bash
    cargo build --release --bin orca-api
    cargo build --release --bin orca-client
    cargo build --release --bin orca-dispatch
    cargo build --release --bin orca-registration
    ```
    Specific executables will be in `target/release/`.

## Installation

After building, you can install the binaries to make them easily accessible from any directory.

### Windows Installation

1.  **Copy Binaries:** Copy the compiled executables (from `target/release/` or `target/debug/`) to a directory included in your system's PATH environment variable (e.g., `C:\Windows\System32`, or a custom directory you've added to PATH).
    *   `orca-api.exe`
    *   `orca-client.exe`
    *   `orca-dispatch.exe`
    *   `orca-registration.exe`
2.  **Configuration Files:** Place the `api.json`, `client.json`, `dispatch.json`, and `registration.json` files in the same directory as the executables, or in a well-known location where your applications can find them (e.g., `C:\ProgramData\Orca` or `C:\Program Files\Orca`). Ensure the paths within these JSON files are correct relative to their new location or are absolute paths.
3.  **Start Services:** You can start the binaries from the command line. For persistent services, consider using Windows Services or Task Scheduler.

### Linux Installation

1.  **Copy Binaries:** Copy the compiled executables (from `target/release/` or `target/debug/`) to a directory in your system's PATH (e.g., `/usr/local/bin`).
    ```bash
    sudo cp target/release/orca-api /usr/local/bin/
    sudo cp target/release/orca-client /usr/local/bin/
    sudo cp target/release/orca-dispatch /usr/local/bin/
    sudo cp target/release/orca-registration /usr/local/bin/
    ```
2.  **Configuration Files:** Place the `api.json`, `client.json`, `dispatch.json`, and `registration.json` files in a suitable configuration directory (e.g., `/etc/orca/` or `/opt/orca/etc/`).
3.  **Start Services:** For persistent services, it's recommended to create systemd service units.

## Configuration Files

Each server component (`orca-api`, `orca-client`, `orca-dispatch`, `orca-registration`) relies on a corresponding JSON configuration file (e.g., `api.json`, `client.json`, `dispatch.json`, `registration.json`). These files must be present in the directory from which the binary is executed, or in a location where the binary is configured to find them.

*   `api.json`:
*   `client.json`:
*   `dispatch.json`:
*   `registration.json`:

Refer to the [Orca Binaries Documentation](binaries.md) for detailed information on each binary's configuration options.

## Running the Project

### Running from Source (Development)

This method is ideal for development as it automatically rebuilds changed code.

*   **Run Registration Server:**
    ```bash
    cargo run --bin orca-registration
    ```
*   **Run API Server:**
    ```bash
    cargo run --bin orca-api
    ```
*   **Run Client (on target machine or another terminal):**
    ```bash
    cargo run --bin orca-client
    ```
*   **Run Dispatcher (CLI):**
    ```bash
    cargo run --bin orca-dispatch -- --command "echo Hello" --client "127.0.0.1"
    ```
    (Remember to use `--` to separate Cargo arguments from your binary's arguments.)

### Running Installed Binaries

Once installed, you can run the binaries directly by their names from any terminal. Ensure your configuration files are accessible.

*   **Run Registration Server:**
    ```bash
    orca-registration
    ```
*   **Run API Server:**
    ```bash
    orca-api
    ```
*   **Run Client:**
    ```bash
    orca-client
    ```
*   **Run Dispatcher (CLI):**
    ```bash
    orca-dispatch --command "echo Hello" --client "127.0.0.1"
    ```