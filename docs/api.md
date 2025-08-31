# Orca API Documentation

This document provides instructions on how to interact with the Orca REST API.

## Endpoint: `/dispatch`

This endpoint is used to dispatch a command to a client.

*   **Method:** `POST`
*   **URL:** `/dispatch`
*   **Headers:** `Content-Type: application/json`

### Request Body

The request body must be a JSON object with the following fields:

| Field     | Type   | Description                                |
| :-------- | :----- | :----------------------------------------- |
| `command` | string | The command to be executed on the client.  |
| `client`  | string | The IP address of the client to target.    |

**Example:**
```json
{
    "command": "ping 8.8.8.8",
    "client": "192.168.2.58"
}
```

### Response Body

The response body will be a JSON object with the following fields:

| Field    | Type    | Description                                           |
| :------- | :------ | :---------------------------------------------------- |
| `stdout` | string  | The standard output from the executed command.        |
| `stderr` | string  | The standard error from the executed command.         |
| `success`| boolean | `true` if the command executed successfully, `false` otherwise. |

**Example:**
```json
{
    "stdout": "Pinging 8.8.8.8 with 32 bytes of data:\nReply from 8.8.8.8: bytes=32 time=10ms TTL=117\n...",
    "stderr": "",
    "success": true
}
```

## Examples

Below are examples of how to call the API using PowerShell and Python. Replace `http://127.0.0.1:8082` with the actual address of your API server if it's different.

### PowerShell

```powershell
$headers = @{
    "Content-Type" = "application/json"
}

$body = @{
    command = "ping 8.8.8.8"
    client = "192.168.2.58"
} | ConvertTo-Json

$response = Invoke-WebRequest -Uri http://127.0.0.1:8082/dispatch -Method POST -Headers $headers -Body $body

# View the response content
$response.Content | ConvertFrom-Json
```

### Python

This example uses the `requests` library. If you don't have it installed, you can install it with `pip install requests`.

```python
import requests
import json

api_url = "http://127.0.0.1:8082/dispatch"
headers = {"Content-Type": "application/json"}
data = {
    "command": "ping 8.8.8.8",
    "client": "192.168.2.58"
}

try:
    response = requests.post(api_url, headers=headers, data=json.dumps(data))
    response.raise_for_status()  # Raise an exception for bad status codes (4xx or 5xx)

    response_data = response.json()
    print("STDOUT:")
    print(response_data.get("stdout"))
    print("\nSTDERR:")
    print(response_data.get("stderr"))
    print(f"\nSuccess: {response_data.get('success')}")

except requests.exceptions.RequestException as e:
    print(f"An error occurred: {e}")

```
