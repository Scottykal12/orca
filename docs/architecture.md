# Orca Project Architecture

## Table of Contents

*   [Core Components](#core-components)
    *   [`orca-registration` (Registration Server)](#orca-registration-registration-server)
    *   [`orca-client` (Client Agent)](#orca-client-client-agent)
    *   [`orca-dispatch` (Command Dispatcher)](#orca-dispatch-command-dispatcher)
    *   [`orca-api` (REST API Server)](#orca-api-rest-api-server)
*   [Workflow and Interactions](#workflow-and-interactions)
    *   [Client Startup and Registration](#client-startup-and-registration)
    *   [Command Dispatch (via `orca-dispatch` CLI)](#command-dispatch-via-orca-dispatch-cli)
    *   [Command Dispatch (via `orca-api`)](#command-dispatch-via-orca-api)
    *   [Client Command Execution](#client-command-execution)
    *   [Event Logging](#event-logging)
*   [Communication Protocols](#communication-protocols)
*   [Database Usage](#database-usage)

This document outlines the overall architecture and how the different components of the Orca project interact with each other.

## Core Components

The Orca project is composed of several distinct binaries, each serving a specific role:

1.  **`orca-registration` (Registration Server):**
    *   **Role:** Manages client registration and stores client metadata (UUID, IP, hostname, MAC address) in a MySQL database.
    *   **Communication:** Clients connect to this server to register or update their information.

2.  **`orca-client` (Client Agent):**
    *   **Role:** Runs on target machines, registers with the `orca-registration` server, and listens for commands from the `orca-dispatch` (or indirectly from `orca-api`).
    *   **Communication:** Connects to `orca-registration` for initial setup. Listens on a specified port for commands and files from `orca-dispatch`.

3.  **`orca-dispatch` (Command Dispatcher):**
    *   **Role:** A command-line tool responsible for sending commands and files directly to `orca-client` instances.
    *   **Communication:** Connects directly to `orca-client` via TCP. Queries the database (via `orca-registration`'s database) to find client connection details.

4.  **`orca-api` (REST API Server):**
    *   **Role:** Provides a web-based interface (REST API) for dispatching commands. It acts as a front-end to the `orca-dispatch` functionality.
    *   **Communication:** Receives HTTP requests from web clients (like the debug webpages). Spawns `orca-dispatch` as a subprocess to handle the actual command and file transfer to the `orca-client`.

## Workflow and Interactions

The typical workflow within the Orca system is as follows:

1.  **Client Startup and Registration:**
    *   An `orca-client` starts up on a target machine.
    *   It connects to the `orca-registration` server.
    *   The `orca-registration` server records or updates the client's information (UUID, IP, hostname, MAC address) in its MySQL database.

2.  **Command Dispatch (via `orca-dispatch` CLI):**
    *   A user runs the `orca-dispatch` command-line tool, specifying a command, a client identifier (UUID, IP, hostname, or MAC address), and optionally a list of files.
    *   `orca-dispatch` queries the MySQL database (using the same database as `orca-registration`) to resolve the client identifier to an IP address.
    *   `orca-dispatch` establishes a TCP connection to the target `orca-client`.
    *   It constructs a `DispatchMessage` (containing the command and file data) and sends it to the `orca-client`.

3.  **Command Dispatch (via `orca-api`):**
    *   A web client (e.g., `dispatcher.html`) sends an HTTP `POST` request to the `orca-api`'s `/dispatch` endpoint with a command, client identifier, and optional file paths.
    *   The `orca-api` server receives this request.
    *   It then spawns `orca-dispatch` as a subprocess, passing the command, client, and file paths as command-line arguments.
    *   `orca-dispatch` then proceeds as described in step 2.

4.  **Client Command Execution:**
    *   The `orca-client` receives the `DispatchMessage` over its TCP connection.
    *   It deserializes the message.
    *   It creates a temporary `orca-workspace` directory (or uses a configured one).
    *   Any received files are saved into this `orca-workspace` directory.
    *   The specified command is executed within the `orca-workspace` directory.
    *   The standard output and standard error from the command execution are sent back to the `orca-dispatch` (or `orca-api` indirectly).
    *   The `orca-workspace` directory and its contents are automatically cleaned up after execution.

5.  **Event Logging:**
    *   After a command is dispatched and a response is received (or an error occurs), `orca-dispatch` logs the event details (epoch time, client UUID, client IP, command, response, and file metadata) into the `events` table in the MySQL database.

## Communication Protocols

*   **Client-Registration:** Simple TCP-based communication for sending client info and receiving registration status.
*   **Dispatch-Client:** Custom TCP-based protocol. A JSON-serialized `DispatchMessage` is sent, followed by a newline delimiter (`\n`). This connection can be secured with Mutual TLS (see [Security](#security) section below).
*   **API-Web Client:** Standard HTTP/HTTPS (RESTful API).
*   **API-Dispatch:** `orca-api` communicates with `orca-dispatch` via command-line arguments and standard I/O (stdout/stderr) for results.

## Security

### Mutual TLS (mTLS)

The connection between the `orca-dispatch` and `orca-client` components can be secured using Mutual TLS (mTLS), providing both encryption and two-way authentication. This is the recommended configuration for production environments.

*   **Activation:** mTLS is enabled by setting the `use_tls` flag to `true` in both `client.json` and `dispatch.json`.
*   **Trust Model:** The system is designed to use the operating system's native certificate trust store. This means that the root Certificate Authority (CA) that issues the client and dispatch certificates must be trusted by the operating systems on both machines.
*   **Roles:**
    *   The `orca-client` acts as the TLS server.
    *   The `orca-dispatch` acts as the TLS client.
*   **Verification:** When mTLS is enabled:
    1.  The `orca-dispatch` (TLS client) verifies the `orca-client`'s (TLS server) certificate against the OS trust store.
    2.  The `orca-client` (TLS server) requires the `orca-dispatch` (TLS client) to present its own certificate, which it also verifies against the OS trust store.

This ensures that only trusted dispatchers can connect to clients, and that dispatchers are connecting to legitimate clients.

## Database Usage


The project utilizes a MySQL database for:

*   **Client Management:** Storing and retrieving `ClientInfo` (UUID, IP, hostname, MAC address).
*   **Event Logging:** Recording `events` of dispatched commands, their responses, and associated file metadata.