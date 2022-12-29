// I am sorry, for I have sinned.

import "./style.scss";
import "xterm/css/xterm.css";
import { FitAddon } from "xterm-addon-fit";
import init, { interpret, validate } from "../public/aps_core";

import { Terminal } from "xterm";

import loader, { Monaco } from "@monaco-editor/loader";

const darkMode = window.matchMedia("(prefers-color-scheme: dark)").matches;

if (darkMode) {
  document.body.classList.add("dark");
}

const $buttons = {
  examples: document.querySelector<HTMLElement>(".actions .examples")!,
  run: document.querySelector<HTMLElement>(".actions .run")!,
  share: document.querySelector<HTMLElement>(".actions .share")!,
};

const $term = document.querySelector<HTMLElement>(".terminal")!;
var term = new Terminal({
  fontFamily: "Roboto Mono",
  fontSize: 15,
  theme: darkMode
    ? {
      background: "#1b1b1b",
    }
    : {
      background: "#EBEBEB",
      foreground: "#333",
      cursor: "#555",
      selectionForeground: "#ccc",
      selectionBackground: "#222",
    },
});

const fitAddon = new FitAddon();
setTimeout(() => {
  term.loadAddon(fitAddon);
  fitAddon.fit();
}, 200);

term.open($term);
const welcomeMessage =
  "Welcome! Use Shift + Enter or press the Run button to run your code. The terminal is readonly and updates automatically.\r\n\r\n$ ";
term.write(welcomeMessage);

// term.onKey((e) => {
//   if (e.key.charCodeAt(0) == 13) {
//     term.write("\n");
//   }
//   term.write(e.key);
// });

init("/aps_core_bg.wasm").catch((e) => {
  alert("Failed to load WASM file. Try reloading the page.");
  console.error(e);
});

let monaco!: Monaco;
let editor!: any;
loader.init().then((mon) => {
  monaco = mon;

  editor = monaco.editor.create(document.querySelector("#editor")!, {
    autoIndent: "full",
    fontFamily: "Roboto Mono",
    readOnly: false,
    fontSize: 16,
    minimap: {
      enabled: false,
    },
    theme: darkMode ? "vs-dark" : "vs-light",
    language: "coffeescript",
    value: 'DISPLAY("Hello, world!")',
  });

  maybeLoadCodeURL(location as any);

  editor.addAction({
    id: "aps.run",
    label: "Run APScript",
    keybindings: [
      monaco.KeyMod.Shift | monaco.KeyCode.Enter,
    ],
    run,
  });

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
});

const old = console.log;
console.log = (...args) => {
  if (!args?.[0]?.startsWith?.("[vite]")) {
    term.writeln(
      args.reduce((acc, v) => acc + v.replaceAll("\\n", "\r\n"), ""),
    );
  }
  old(...args);
};

const oldPrompt = window.prompt;

window.prompt = (msg) => {
  term.write(msg as any);
  const result = oldPrompt(msg);
  term.writeln(" " + result);
  return result;
};

function maybeLoadCodeURL(url: URL) {
  if (!url.hash) return;

  term.clear();
  term.write("\r" + welcomeMessage);

  if (url.hash.startsWith("#code/")) {
    try {
      editor.setValue(atob(url.hash.slice("#code/".length)));
    } catch {
      editor.setValue('DISPLAY("Hello, world!")');
    }
  } else if (url.hash.startsWith("#examples/")) {
    const id = url.hash.slice("#examples/".length);

    if (id === "using-lists") {
      editor.setValue(`# This creates a list of names.
names <- ["John", "Jim", "Jerry"]

# You can access the first name, John, by using the bracket notation:
firstName <- names[1]
# Note that indices start from 1, 2, ...
DISPLAY(firstName)

# You can also add new names by calling the APPEND function:
APPEND(names, "Justin")

# And let's view our final list of names:
DISPLAY(names)`);
    } else if (id === "basic-variables") {
      editor.setValue(`# Variables allow you to store data into named locations.
# The following statement assigns the number 23 to 'foo'.
foo <- 23

# You can access old variables when creating new ones.
bar <- foo + 5 # <= 28

# Furthermore, you can re-assign variables. The old value is forgotten.
foo <- bar + 5 # 33`);
    } else if (id === "defining-procedures") {
      editor.setValue(
        `# A procedure allows you to run the same code over and over
# with different variables (parameters). The following code
# defines a procedure named add which takes in a variable
# 'a' and 'b'.
PROCEDURE add(a, b) {
	# The return statement replaces the call expression 'add(a, b)'
	# with the corresponding value returned.
	RETURN a + b
}

DISPLAY(add(2, 5))
DISPLAY(add(6, 2))
DISPLAY(add(-2, 5))`,
      );
    } else if (id === "iterating-lists") {
      editor.setValue(`# Create a list of colors.
colors <- ["red", "green", "blue"]

FOR EACH color IN colors {
	DISPLAY(color)
}`);
    } else {
      editor.setValue('DISPLAY("Hello, world!")');
    }
  }
}

$buttons.run.addEventListener("click", run);
$buttons.share.addEventListener("click", () => {
  window.location.hash = "code/" + btoa(editor.getValue());
  navigator.clipboard.writeText(window.location.href).catch(console.error);
  // TODO: show breadcrumb
});

const markers: any[] = [];

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

const $overlay = document.querySelector<HTMLElement>("#overlay")!;

$buttons.examples.addEventListener("click", () => {
  $overlay.classList.add("showing");
});

$overlay.addEventListener("click", (e) => {
  if (e.target === $overlay) {
    $overlay.classList.remove("showing");
  }
});

document.querySelectorAll<HTMLAnchorElement>("#overlay .content a").forEach(
  (a) => {
    a.addEventListener("click", () => {
      maybeLoadCodeURL(new URL(a.href));
      $overlay.classList.remove("showing");
    });
  },
);
