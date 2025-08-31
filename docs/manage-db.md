# Database Management Script (`manage-db.ps1`)

This document provides instructions on how to use the `manage-db.ps1` PowerShell script to create and destroy the MySQL database used by the Orca project.

## Table of Contents

*   [Overview](#overview)
*   [Prerequisites](#prerequisites)
*   [Functions](#functions)
    *   [`Create-Database`](#create-database)
    *   [`Destroy-Database`](#destroy-database)
*   [Usage Examples](#usage-examples)

## Overview

The `manage-db.ps1` script is a PowerShell script designed to simplify the setup and teardown of the `orca_db` MySQL database for development and testing purposes. It includes functions to create the database and its necessary tables (`clients` and `events`), and to completely remove the database.

## Prerequisites

*   **PowerShell:** The script requires PowerShell to be installed on your system.
*   **MySQL Client:** The `mysql` command-line client must be installed and accessible in your system's PATH. This client is used by the script to interact with the MySQL server.
*   **MySQL Server:** A running MySQL server instance must be accessible, typically on `localhost:3306` with a `root` user and `password` (default credentials used by the script).

## Functions

### `Create-Database`

This function creates the `orca_db` database and the `clients` and `events` tables within it. It is idempotent, meaning it will not throw an error if the database or tables already exist.

#### Parameters

*   `DbName` (string, optional): The name of the database to create. Defaults to `orca_db`.
*   `DbUser` (string, optional): The MySQL user to connect as. Defaults to `root`.
*   `DbPass` (string, optional): The password for the MySQL user. Defaults to `password`.
*   `DbHost` (string, optional): The hostname of the MySQL server. Defaults to `localhost`.
*   `DbPort` (string, optional): The port of the MySQL server. Defaults to `3306`.

#### Usage

To create the database with default settings:

```powershell
. .\manage-db.ps1
Create-Database
```

To create the database with custom settings:

```powershell
. .\manage-db.ps1
Create-Database -DbName "my_custom_db" -DbUser "admin" -DbPass "mysecret" -DbHost "192.168.1.10" -DbPort "3307"
```

### `Destroy-Database`

This function drops (deletes) the `orca_db` database. Use with caution, as this will permanently remove all data within the database.

#### Parameters

*   `DbName` (string, optional): The name of the database to destroy. Defaults to `orca_db`.
*   `DbUser` (string, optional): The MySQL user to connect as. Defaults to `root`.
*   `DbPass` (string, optional): The password for the MySQL user. Defaults to `password`.
*   `DbHost` (string, optional): The hostname of the MySQL server. Defaults to `localhost`.
*   `DbPort` (string, optional): The port of the MySQL server. Defaults to `3306`.

#### Usage

To destroy the database with default settings:

```powershell
. .\manage-db.ps1
Destroy-Database
```

To destroy the database with custom settings:

```powershell
. .\manage-db.ps1
Destroy-Database -DbName "my_custom_db" -DbUser "admin" -DbPass "mysecret" -DbHost "192.168.1.10" -DbPort "3307"
```

## Usage Examples

```powershell
# Source the script (run from the project root)
. .\manage-db.ps1

# Create the database and tables
Create-Database

# ... run your Orca components ...

# Destroy the database
Destroy-Database
```
