# T3 Chat Export Viewer

T3 Chat Export Viewer is a desktop application built using [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe). It allows users to drag and drop T3 JSON files, view their contents in a structured format, and interact with the data.


https://github.com/user-attachments/assets/9303adff-0c3e-4400-ae5b-a67e62e938dd


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


## Acknowledgments

- Built with [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).
- Thanks Theo for making t3.chat

## Side notes

Ctrl+f in devtools sources for "Exporting your chat data..." to find the download logic

```js
        async function e1() {
            return _.oR.promise(async () => {
                let e = new Blob([JSON.stringify({
                    threads: await B.threads.toArray(),
                    messages: await B.messages.toArray()
                })],{
                    type: "application/json"
                })
                  , t = URL.createObjectURL(e)
                  , s = document.createElement("a");
                return s.href = t,
                s.download = "t3chat-export-".concat(new Date().toISOString(), ".json"),
                document.body.appendChild(s),
                s.click(),
                document.body.removeChild(s),
                URL.revokeObjectURL(t),
                {
                    success: !0
                }
            }
            , {
                loading: "Exporting your chat data...",
                success: {
                    message: "Export completed successfully",
                    description: "Your chat data has been exported.",
                    duration: 5e3
                },
                error: e => (console.error("[IMPORT-EXPORT] Error exporting data:", e),
                {
                    message: "Export failed",
                    description: "There was an error exporting your chat data.",
                    duration: 1 / 0,
                    closeButton: !0
                })
            })
        }
```

we can see that the payload is constructed using 

```js
JSON.stringify({
   threads: await B.threads.toArray(),
   messages: await B.messages.toArray()
})
```

and I remember in a video Theo mentioning that the db is exposed in the devtools

There's a global `dxdb` object, so we can call

```js
copy(JSON.stringify({
   threads: await dxdb.threads.toArray(),
   messages: await dxdb.messages.toArray(),
}, null, 2))
```

which puts in our clipboard a pretty-printed JSON export, the same as if we had clicked the export button.c