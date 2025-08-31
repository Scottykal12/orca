# Debug Website Setup and Usage

This document provides instructions on how to set up and use the Orca debug webpages located in the `debug-website` directory.

## Table of Contents

*   [Overview](#overview)
*   [Prerequisites](#prerequisites)
*   [Serving the Debug Website](#serving-the-debug-website)
    *   [Option 1: Opening Files Directly (Simple)](#option-1-opening-files-directly-simple)
    *   [Option 2: Using a Local Web Server (Recommended)](#option-2-using-a-local-web-server-recommended)
*   [Using the Debug Webpages](#using-the-debug-webpages)
    *   [`index.html` (Debug Tools Home)](#indexhtml-debug-tools-home)
    *   [`dispatcher.html` (API Dispatcher)](#dispatcherhtml-api-dispatcher)

## Overview

The debug webpages are a set of simple HTML, CSS, and JavaScript files designed to help you test and interact with the Orca API and components during development. They are located in the `debug-website` folder in the project root.

**Important Security Note:** The `index.html` page contains a prominent warning about direct database connections from the browser. This is a critical security vulnerability and is included for demonstration purposes only. **Never expose database credentials directly in client-side code in a production environment.** All secure database interactions should occur via a backend service.

## Prerequisites

To use the debug webpages effectively, ensure the following:

*   **Orca API Server Running:** The `orca-api` server must be running and accessible. It should be built with CORS support enabled (as configured during development).
    *   To start: `cargo run --bin orca-api`
*   **Orca Client Running:** If you intend to dispatch commands, an `orca-client` instance must be running and registered.
    *   To start: `cargo run --bin orca-client`
*   **Web Browser:** A modern web browser (e.g., Chrome, Firefox, Edge).
*   **Optional: Node.js and `live-server`:** For a more convenient development experience, a simple local web server like `live-server` (a Node.js package) is recommended. Install it globally:
    ```bash
    npm install -g live-server
    ```

## Serving the Debug Website

### Option 1: Opening Files Directly (Simple)

You can simply open the `index.html` file in your web browser. This is the easiest method but might have limitations due to browser security policies (e.g., `file://` protocol restrictions on AJAX requests).

Navigate to the `debug-website` folder and double-click `index.html`, or open it via your browser's file menu:

`file:///path/to/your/project/orca/debug-website/index.html`

(Replace `/path/to/your/project/orca/` with the actual path to your Orca project directory).

### Option 2: Using a Local Web Server (Recommended)

Using a local web server provides a more robust environment, avoiding `file://` protocol restrictions and enabling features like live reloading.

1.  **Navigate to `debug-website`:** Open your terminal and change the directory to `debug-website`:
    ```bash
    cd debug-website
    ```
2.  **Start `live-server`:**
    ```bash
    live-server
    ```
    This will typically open your default browser to `http://127.0.0.1:8080` (or another available port) serving the `index.html` file.

## Using the Debug Webpages

### `index.html` (Debug Tools Home)

This is the landing page for all Orca debug tools. It provides a central place to navigate to different debugging functionalities.

*   **Purpose:** Lists available debug tools.
*   **Features:**
    *   Displays the Orca logo (colored red to indicate debug status).
    *   Provides links to other debug pages.

### `dispatcher.html` (API Dispatcher)

This page allows you to send commands and files to an `orca-client` via the `orca-api`.

#### How to Use

1.  **Ensure `orca-api` is Running:** As mentioned in prerequisites.
2.  **Open `dispatcher.html`:** Access it via `index.html` or directly through your local web server (e.g., `http://127.0.0.1:8080/dispatcher.html`).
3.  **Fill in the Fields:**
    *   **Client IP:** Enter the IP address of the `orca-client` you wish to target.
    *   **Command:** Enter the command string you want to execute.
    *   **Files (comma-separated paths):** (Optional) Enter a comma-separated list of file paths from your local machine. These files will be sent to the client's workspace directory.
4.  **Click "Dispatch":** The page will send a `POST` request to the `orca-api`'s `/dispatch` endpoint.

#### Response

The response from the `orca-api` (including `stdout`, `stderr`, and `success` status from the client's command execution) will be displayed on the page.
