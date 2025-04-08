# T3 Chat Export Viewer

T3 Chat Export Viewer is a desktop application built using [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe). It allows users to drag and drop T3 JSON files, view their contents in a structured format, and interact with the data.

## Features

- **Drag-and-Drop Support**: Easily drop T3 JSON files into the application.
- **Thread Viewer**: Displays threads in an expandable format.
- **Message Viewer**: View messages within threads, truncated to 256 characters for readability.
- **Copy to Clipboard**: Copy thread data and associated messages as JSON with a single click.
- **Dynamic UI**: Each dropped file opens in its own window, and closing a window removes the corresponding file.

## How It Works

1. **Drag and Drop**: Drop a T3 JSON file into the application window.
2. **Thread Display**: Each thread is displayed as an expandable section.
3. **Message Display**: Messages within a thread are shown when the thread is expanded.
4. **Copy Functionality**: Use the "Copy" button next to a thread title to copy the thread and its messages as JSON to the clipboard.

## Installation

1. Ensure you have Rust installed. If not, install it from [rust-lang.org](https://www.rust-lang.org/).
2. Clone the repository:
   ```sh
   git clone https://github.com/your-username/t3-chat-export-viewer.git
   cd t3-chat-export-viewer
   ```
3. Build and run the application:
   ```sh
   cargo run
   ```

## Usage

1. Launch the application.
2. Drag and drop a T3 JSON file into the application window.
3. Interact with the threads and messages:
   - Expand threads to view their details and messages.
   - Use the "Copy" button to copy thread data and messages as JSON.

## Screenshots

![Screenshot](screenshot.png)

## Acknowledgments

- Built with [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).
