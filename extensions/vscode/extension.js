const vscode = require("vscode");
const cp = require("child_process");

const SELECTION_DEBOUNCE_MS = 100;
const OSX_TERMINAL_ENV_SETTING = "terminal.integrated.env.osx";
const NEW_SESSION_ENV_VALUE = "1";

// Pending debounced selection event. Terminal focus events cancel it, so a late
// editor-focus event cannot override a newer terminal-focus event.
let selectionTimer;

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
// The setting can be unset, so fall back to an empty object. Read the global value only, so that
// workspace values are not copied into the global settings. Only write when the value changes.
let osxEnv = config.inspect(OSX_TERMINAL_ENV_SETTING)?.globalValue ?? {};
if (osxEnv.TAB_NEW_SESSION !== NEW_SESSION_ENV_VALUE) {
  config
    .update(
      OSX_TERMINAL_ENV_SETTING,
      { ...osxEnv, TAB_NEW_SESSION: NEW_SESSION_ENV_VALUE },
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
  clearTimeout(selectionTimer);
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

function deactivate() {
  clearTimeout(selectionTimer);
}

module.exports = {
  activate,
  deactivate,
};
