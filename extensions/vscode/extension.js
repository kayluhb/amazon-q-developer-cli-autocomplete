const vscode = require("vscode");
const cp = require("child_process");

const SELECTION_DEBOUNCE_MS = 100;

let runCommand = function (command) {
  cp.exec(command, (err) => {
    if (err) {
      logError(err);
    }
  });
};

let config = vscode.workspace.getConfiguration();
// fig.log is an internal setting as it's intended for developers only, this means it won't show up
// in the settings UI/editor. Add `"fig.log": true` to your settings.json and reload the window to
// enable logging.
let shouldLog = config.get("q.log") === true;

// Ensure that any VSCode terminal session has TAB_NEW_SESSION set as an environment variable
// The setting can be unset, so fall back to an empty object. Only write the global settings when the
// value changes.
let osxEnv = config.get("terminal.integrated.env.osx") ?? {};
if (osxEnv["TAB_NEW_SESSION"] !== "1") {
  config
    .update(
      "terminal.integrated.env.osx",
      { ...osxEnv, TAB_NEW_SESSION: "1" },
      true,
    )
    .then(undefined, logError);
}

function log(...args) {
  if (shouldLog) {
    console.log(`q: ${args[0]}`, args.slice(1));
  }
}

function logError(message) {
  console.error(`q: ${message}`);
}

function updateActiveTerminal(terminal) {
  let activeTerminal = terminal || vscode.window.activeTerminal;

  if (!activeTerminal) {
    noActiveTerminals();
    return;
  }
  activeTerminal.processId.then((processId) => {
    if (processId) {
      runCommand(
        `tab hook keyboard-focus-changed ${vscode.env.uriScheme} ${processId}`,
      );
    }
  });
}

function noActiveTerminals() {
  log("no active terminals");
}

console.log(vscode.env.uriScheme);

function activate() {
  try {
    updateActiveTerminal();

    vscode.window.onDidOpenTerminal(() => {
      log("Terminal opened. Total count: " + vscode.window.terminals.length);
    });

    vscode.window.onDidCloseTerminal(() => {
      log("Terminal closed. Total count: " + vscode.window.terminals.length);

      if (vscode.window.terminals.length == 0) {
        noActiveTerminals();
      } else {
        updateActiveTerminal();
      }
    });

    vscode.window.onDidChangeActiveTerminal((terminal) => {
      log(`Active terminal changed`);
      updateActiveTerminal(terminal);
    });

    // Selection changes fire on every cursor move. Debounce them so that we start one `tab`
    // process per burst instead of one per event.
    let selectionTimer;
    vscode.window.onDidChangeTextEditorSelection(() => {
      clearTimeout(selectionTimer);
      selectionTimer = setTimeout(() => {
        runCommand(`tab hook keyboard-focus-changed ${vscode.env.uriScheme} 0`);
      }, SELECTION_DEBOUNCE_MS);
    });
  } catch (e) {
    logError(e);
  }
}
exports.activate = activate;

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
