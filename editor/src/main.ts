import "./style.css";
import "xterm/css/xterm.css";
import { FitAddon } from "xterm-addon-fit";
import init, { interpret, validate } from "../lib/aps_core";

import { Terminal } from "xterm";

import loader from "@monaco-editor/loader";

const $buttons = {
  examples: document.querySelector<HTMLElement>(".actions .examples")!,
  run: document.querySelector<HTMLElement>(".actions .run")!,
};

const $term = document.querySelector<HTMLElement>(".terminal")!;
var term = new Terminal({
  fontFamily: "Roboto Mono",
  fontSize: 15,
});

const fitAddon = new FitAddon();
setTimeout(() => {
  term.loadAddon(fitAddon);
  fitAddon.fit();
}, 200);

term.open($term);
term.write(
  "Welcome! Use Shift + Enter to quickly run your code.\r\n\r\n$ ",
);

term.onKey((e) => {
  if (e.key.charCodeAt(0) == 13) {
    term.write("\n");
  }
  term.write(e.key);
});

init("../lib/aps_core_bg.wasm").catch((e) => {
  alert("Failed to load WASM file. Try reloading the page.");
  console.error(e);
});

let monaco = await loader.init();

const old = console.log;
console.log = (...args) => {
  if (!args?.[0]?.startsWith?.("[vite]")) {
    term.writeln(
      args.reduce((acc, v) => acc + v.replaceAll("\\n", "\r\n"), ""),
    );
  }
  old(...args);
};

const editor = monaco.editor.create(document.querySelector("#editor")!, {
  autoIndent: "full",
  fontFamily: "Roboto Mono",
  readOnly: false,
  fontSize: 16,
  minimap: {
    enabled: false,
  },
  language: "coffeescript",
});

$buttons.run.addEventListener("click", run);

editor.addAction({
  id: "aps.run",
  label: "Run APScript",
  keybindings: [
    monaco.KeyMod.Shift | monaco.KeyCode.Enter,
  ],
  run,
});

function run() {
  term.write("\r$ aps run <file>\r\n");
  const result = interpret(editor.getValue());

  markers.length = 0;
  if (result.Data) {
    term.write(result.Data.log.replaceAll("\n", "\r\n"));

    for (let i = 0; i < result.Data.errors.length; i++) {
      const model = editor.getModel()!;
      const start = model.getPositionAt(
        result.Data.errors[i].labels[0].range.start,
      );
      const end = model.getPositionAt(
        result.Data.errors[i].labels[0].range.end,
      );
      markers[i] = {
        startLineNumber: start.lineNumber,
        endLineNumber: end.lineNumber,
        startColumn: start.column,
        endColumn: end.column,
        message: result.Data.errors[i].message,
        severity: monaco.MarkerSeverity.Error,
      };
    }

    monaco.editor.setModelMarkers(editor.getModel()!, "owner", markers);
  }
  term.write("$ ");
}

$buttons.examples.addEventListener("click", () => {
  alert("Examples");
});

const markers: monaco.editor.IMarkerData[] = [];

editor.onDidChangeModelContent(() => {
  const errors = validate(editor.getValue());

  markers.length = errors?.length ?? 0;
  if (!errors) return;

  for (let i = 0; i < errors.length; i++) {
    const model = editor.getModel()!;
    const start = model.getPositionAt(errors[i].labels[0].range.start);
    const end = model.getPositionAt(errors[i].labels[0].range.end);
    markers[i] = {
      startLineNumber: start.lineNumber,
      endLineNumber: end.lineNumber,
      startColumn: start.column,
      endColumn: end.column,
      message: errors[i].message,
      severity: i === 0
        ? monaco.MarkerSeverity.Error
        : monaco.MarkerSeverity.Info,
    };
  }

  monaco.editor.setModelMarkers(editor.getModel()!, "owner", markers);
});
