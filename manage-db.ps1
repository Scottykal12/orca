# This script provides functions to create and destroy a temporary MySQL database for testing.

# Function to create the database and tables
function Create-Database {
    param (
        [string]$DbName = "orca_db",
        [string]$DbUser = "root", # Common MySQL default user
        [string]$DbPass = "password",
        [string]$DbHost = "localhost",
        [string]$DbPort = "3306" # Default MySQL port
    )

    # Check if mysql client is available
    $mysql = Get-Command mysql -ErrorAction SilentlyContinue
    if (-not $mysql) {
        Write-Error "mysql command not found. Please install MySQL client and make sure it's in your PATH."
        return
    }

    # Build connection arguments for mysql client
    $connArgs = @("-h", $DbHost, "-P", $DbPort, "-u", $DbUser)
    if ($DbPass) {
        $connArgs += "--password=$DbPass"
    }

    Write-Host "Attempting to create database '$DbName'..."

    # Create the database
    try {
        & $mysql $connArgs -e "CREATE DATABASE IF NOT EXISTS $DbName;"
        Write-Host "Database '$DbName' created successfully."
    } catch {
        Write-Warning "Database '$DbName' might already exist or there was an error creating it: $($_.Exception.Message)"
    }

    Write-Host "Attempting to create tables in database '$DbName'..."

    # Create clients table
    $sqlClients = @"
    CREATE TABLE IF NOT EXISTS clients (
        uuid VARCHAR(255) PRIMARY KEY,
        ip VARCHAR(255),
        hostname VARCHAR(255),
        mac_address VARCHAR(255)
    );
"@
    try {
        & $mysql $connArgs $DbName -e $sqlClients
        Write-Host "Table 'clients' created successfully."
    } catch {
        Write-Error "Error creating 'clients' table: $($_.Exception.Message)"
        return
    }

    # Create events table
    $sqlEvents = @"
    CREATE TABLE IF NOT EXISTS events (
        id INT AUTO_INCREMENT PRIMARY KEY,
        epoch_time BIGINT,
        client_uuid VARCHAR(255),
        client_ip VARCHAR(255),
        command TEXT,
        response TEXT,
        files TEXT
    );
"@
    try {
        & $mysql $connArgs $DbName -e $sqlEvents
        Write-Host "Table 'events' created successfully."
    } catch {
        Write-Error "Error creating 'events' table: $($_.Exception.Message)"
        return
    }

    # Create logs table
    $sqlLogs = @"
    CREATE TABLE IF NOT EXISTS logs (
        id INT AUTO_INCREMENT PRIMARY KEY,
        time BIGINT,
        service TEXT,
        severity TEXT,
        info TEXT
    );
"@
    try {
        & $mysql $connArgs $DbName -e $sqlLogs
        Write-Host "Table 'logs' created successfully."
    } catch {
        Write-Error "Error creating 'logs' table: $($_.Exception.Message)"
        return
    }

    Write-Host "Database '$DbName' and tables created successfully."
}

# Function to destroy the database
function Destroy-Database {
    param (
        [string]$DbName = "orca_db",
        [string]$DbUser = "root",
        [string]$DbPass = "password",
        [string]$DbHost = "localhost",
        [string]$DbPort = "3306"
    )

    # Check if mysql client is available
    $mysql = Get-Command mysql -ErrorAction SilentlyContinue
    if (-not $mysql) {
        Write-Error "mysql command not found. Please install MySQL client and make sure it's in your PATH."
        return
    }

    # Build connection arguments for mysql client
    $connArgs = @("-h", $DbHost, "-P", $DbPort, "-u", $DbUser)
    if ($DbPass) {
        $connArgs += "--password=$DbPass"
    }

    Write-Host "Attempting to destroy database '$DbName'..."

    # Drop the database
    try {
        & $mysql $connArgs -e "DROP DATABASE IF EXISTS $DbName;"
        Write-Host "Database '$DbName' destroyed successfully."
    } catch {
        Write-Error "Error destroying database '$DbName': $($_.Exception.Message)"
    }
}