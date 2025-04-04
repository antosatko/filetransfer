# File Transfer Client

A simple client application to download binary data from a server, continuously requesting the largest remaining data chunk. It displays a progress bar to inform the user of the download progress. Once the download is complete, the data is saved to a file, and if the user provides a checksum, the application verifies the file's integrity using SHA-256.

## Features

- Continuously downloads the largest remaining data chunk from the server.
- Displays download progress using a loading bar.
- Saves the downloaded data to a file.
- Optionally verifies the SHA-256 checksum for data integrity.  

## Usage

The application takes the following optional arguments:  

| Argument | Description                                             |
|----------|---------------------------------------------------------|
| **-h**   | Show help and usage information.                        |
| **-a**   | Specify the server address (default: `127.0.0.1:8080`). |
| **-o**   | Set the output file path (default: `data`).             |
| **-c**   | Provide a SHA-256 checksum to verify data integrity.    |

### Example

```bash
myftp -a 192.168.1.10:8080 -o download.bin -c d2c76a4a...
```

## Checksum Verification

If a checksum is provided, the application computes the SHA-256 hash of the downloaded data and compares it with the given checksum. Thesult is displayed as either a match or a mismatch.
