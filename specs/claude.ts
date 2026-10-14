// Completion spec for Claude Code (`claude`), written against v2.1.x.
// Not part of withfig/autocomplete: build-scripts/build.py copies it into the specs source tree.

const models: Fig.Suggestion[] = [
  { name: "fable", description: "Latest Fable model" },
  { name: "opus", description: "Latest Opus model" },
  { name: "sonnet", description: "Latest Sonnet model" },
  { name: "haiku", description: "Latest Haiku model" },
];

const scopeNames = ["local", "user", "project"];

// Claude Code stores each project's transcripts in ~/.claude/projects/<cwd with
// every non-alphanumeric character replaced by "-">/<session id>.jsonl
const sessionGenerator: Fig.Generator = {
  custom: async (_tokens, executeShellCommand, context) => {
    const dir = `~/.claude/projects/${context.currentWorkingDirectory.replace(/[^A-Za-z0-9]/g, "-")}`;
    const { stdout } = await executeShellCommand({
      command: "bash",
      args: [
        "-c",
        `{ cd ${dir} && ls -t | grep '\\.jsonl$' | head -50 | xargs stat -f '%N|%Sm' -t '%Y-%m-%d %H:%M'; } 2>/dev/null || true`,
      ],
    });
    return stdout
      .split("\n")
      .filter(Boolean)
      .map((line, index) => {
        const [file, modified] = line.split("|");
        return {
          name: file.replace(/\.jsonl$/, ""),
          description: `Last active ${modified}`,
          priority: 100 - index,
        };
      });
  },
};

// Reads the config files directly: `claude mcp list` health-checks every server and is too slow
const mcpServerGenerator: Fig.Generator = {
  custom: async (_tokens, executeShellCommand, context) => {
    const readJson = async (path: string) => {
      const { stdout } = await executeShellCommand({
        command: "bash",
        args: ["-c", `cat ${path} 2>/dev/null || true`],
        cwd: context.currentWorkingDirectory,
      });
      try {
        return JSON.parse(stdout);
      } catch {
        return {};
      }
    };
    const config = await readJson("~/.claude.json");
    const project = await readJson(".mcp.json");
    const scopes: [string, Record<string, unknown> | undefined][] = [
      ["local", config.projects?.[context.currentWorkingDirectory]?.mcpServers],
      ["project", project.mcpServers],
      ["user", config.mcpServers],
    ];
    // .mcp.json comes from the repo, and the engine does not escape ; $ or newlines on insert
    const safeName = /^[\w.-]+$/;
    return scopes.reduce<Fig.Suggestion[]>(
      (suggestions, [scope, servers]) => [
        ...suggestions,
        ...Object.keys(servers ?? {})
          .filter((name) => safeName.test(name))
          .map((name) => ({ name, description: `MCP server (${scope})` })),
      ],
      []
    );
  },
};

const pluginGenerator: Fig.Generator = {
  script: ["claude", "plugin", "list", "--json"],
  postProcess: (out) => {
    try {
      return JSON.parse(out).map(
        ({
          id,
          version,
          enabled,
        }: {
          id: string;
          version: string;
          enabled: boolean;
        }) => ({
          name: id,
          description: `${version}${enabled ? "" : " (disabled)"}`,
        })
      );
    } catch {
      return [];
    }
  },
};

const backgroundSessionGenerator: Fig.Generator = {
  script: ["claude", "agents", "--json", "--all"],
  postProcess: (out) => {
    try {
      return JSON.parse(out).map(
        ({
          id,
          name,
          state,
          cwd,
        }: {
          id: string;
          name?: string;
          state: string;
          cwd: string;
        }) => ({
          name: id,
          description: `${name ?? cwd} (${state})`,
        })
      );
    } catch {
      return [];
    }
  },
};

const helpOption: Fig.Option = {
  name: ["-h", "--help"],
  description: "Display help for command",
};

const scopeOption: Fig.Option = {
  name: ["-s", "--scope"],
  description: "Configuration scope",
  args: { name: "scope", suggestions: scopeNames },
};

const jsonOption: Fig.Option = {
  name: "--json",
  description: "Output as JSON",
};

const yesOption: Fig.Option = {
  name: ["-y", "--yes"],
  description: "Skip confirmation",
};

const clientSecretOption: Fig.Option = {
  name: "--client-secret",
  description: "Prompt for OAuth client secret",
};

const acceptCommandOption: Fig.Option = {
  name: "--accept-command",
  description: "Accept a plugin command by hash",
  args: { name: "sha256" },
};

const backgroundSessionArg: Fig.Arg = {
  name: "id|name",
  generators: backgroundSessionGenerator,
};

const mcpServerArg: Fig.Arg = { name: "name", generators: mcpServerGenerator };

const pluginArg: Fig.Arg = { name: "plugin", generators: pluginGenerator };

const permissionModes: Fig.Suggestion[] = [
  { name: "acceptEdits", description: "Auto-accept file edits" },
  { name: "auto", description: "Let the classifier decide" },
  { name: "bypassPermissions", description: "Skip all permission checks" },
  { name: "manual", description: "Ask for every action" },
  { name: "dontAsk", description: "Deny anything that would prompt" },
  { name: "plan", description: "Plan only, no edits" },
];

const effortLevels = ["low", "medium", "high", "xhigh", "max"];

// Options shared by the interactive session and `claude agents`
const sessionOptions: Fig.Option[] = [
  {
    name: "--agent",
    description: "Agent for the current session",
    args: { name: "agent" },
  },
  {
    name: "--allow-dangerously-skip-permissions",
    description:
      "Make bypassing permission checks available without enabling it",
  },
  {
    name: "--dangerously-skip-permissions",
    description: "Bypass all permission checks",
    isDangerous: true,
  },
  {
    name: "--effort",
    description: "Effort level for the session",
    args: { name: "level", suggestions: effortLevels },
  },
  {
    name: "--model",
    description: "Model for the session",
    args: { name: "model", suggestions: models },
  },
  {
    name: "--permission-mode",
    description: "Permission mode for the session",
    args: { name: "mode", suggestions: permissionModes },
  },
  {
    name: "--plugin-dir",
    description: "Load a plugin from a directory or .zip for this session",
    args: { name: "path", template: ["folders", "filepaths"] },
  },
  {
    name: "--restricted",
    description: "Remove code-running tools and ignore settings files",
  },
  {
    name: "--setting-sources",
    description: "Comma-separated setting sources to load",
    args: {
      name: "sources",
      suggestions: [
        "user",
        "project",
        "local",
        "user,project",
        "user,project,local",
      ],
    },
  },
  {
    name: "--settings",
    description: "Settings JSON file or string",
    args: { name: "file-or-json", template: "filepaths" },
  },
  {
    name: "--strict-mcp-config",
    description: "Only use MCP servers from --mcp-config",
  },
];

const completionSpec: Fig.Spec = {
  name: "claude",
  description: "Claude Code: agentic coding in your terminal",
  subcommands: [
    {
      name: "agents",
      description: "Manage background agents",
      options: [
        ...sessionOptions,
        {
          name: "--add-dir",
          description: "Additional directory for dispatched sessions",
          args: { name: "directory", template: "folders" },
          isRepeatable: true,
        },
        {
          name: "--mcp-config",
          description: "MCP server configuration for dispatched sessions",
          args: { name: "config", template: "filepaths" },
          isRepeatable: true,
        },
        {
          name: "--all",
          description: "With --json: also include completed sessions",
        },
        {
          name: "--cwd",
          description: "Only sessions started under this path",
          args: { name: "path", template: "folders" },
        },
        jsonOption,
        helpOption,
      ],
    },
    {
      name: "attach",
      description: "Open a background session in this terminal",
      args: backgroundSessionArg,
    },
    {
      name: "auth",
      description: "Manage authentication",
      subcommands: [
        {
          name: "login",
          description: "Log in",
          options: [
            {
              name: "--claudeai",
              description: "Log in with a Claude subscription",
            },
            {
              name: "--console",
              description: "Log in with an Anthropic Console account",
            },
            {
              name: "--email",
              description: "Email address",
              args: { name: "email" },
            },
            { name: "--sso", description: "Log in with SSO" },
            helpOption,
          ],
        },
        { name: "logout", description: "Log out", options: [helpOption] },
        {
          name: "status",
          description: "Show authentication status",
          options: [
            jsonOption,
            { name: "--text", description: "Output as text" },
            helpOption,
          ],
        },
      ],
      options: [helpOption],
    },
    {
      name: "auto-mode",
      description: "Inspect or reset auto mode classifier configuration",
      subcommands: [
        { name: "config", description: "Show the effective configuration" },
        { name: "critique", description: "Critique custom rules" },
        { name: "defaults", description: "Show the default rules" },
        { name: "reset", description: "Reset the configuration" },
      ],
      options: [helpOption],
    },
    {
      name: "doctor",
      description: "Check the health of your Claude Code installation",
      options: [helpOption],
    },
    {
      name: "gateway",
      description: "Run the enterprise auth/telemetry gateway",
      options: [
        {
          name: "--config",
          description: "Path to gateway YAML config",
          args: { name: "path", template: "filepaths" },
        },
        helpOption,
      ],
    },
    {
      name: "import",
      description: "Import config from another AI coding agent",
      args: {
        name: "source",
        isOptional: true,
        suggestions: ["codex", "gemini", "cursor"],
      },
      options: [
        { name: "--dry-run", description: "Show what would be imported" },
        {
          name: "--yes",
          description: "Skip the picker; --yes=<digest> on headless surfaces",
        },
        helpOption,
      ],
    },
    {
      name: "install",
      description: "Install Claude Code native build",
      args: {
        name: "target",
        isOptional: true,
        suggestions: ["stable", "latest"],
      },
      options: [
        { name: "--force", description: "Reinstall even if installed" },
        helpOption,
      ],
    },
    {
      name: "logs",
      description: "Print a background session's recent terminal output",
      args: backgroundSessionArg,
    },
    {
      name: "mcp",
      description: "Configure and manage MCP servers",
      subcommands: [
        {
          name: "add",
          description: "Add an MCP server",
          args: [{ name: "name" }, { name: "commandOrUrl", isCommand: true }],
          options: [
            {
              name: "--callback-port",
              description: "Fixed port for OAuth callback",
              args: { name: "port" },
            },
            {
              name: "--client-id",
              description: "OAuth client ID",
              args: { name: "clientId" },
            },
            clientSecretOption,
            {
              name: ["-e", "--env"],
              description: "Set environment variables (KEY=value)",
              args: { name: "env", isVariadic: true },
            },
            {
              name: ["-H", "--header"],
              description: "Set headers for HTTP/SSE servers",
              args: { name: "header", isVariadic: true },
            },
            scopeOption,
            {
              name: ["-t", "--transport"],
              description: "Transport type",
              args: {
                name: "transport",
                suggestions: ["stdio", "sse", "http"],
              },
            },
            helpOption,
          ],
        },
        {
          name: "add-from-claude-desktop",
          description: "Import MCP servers from Claude Desktop",
          options: [scopeOption, helpOption],
        },
        {
          name: "add-json",
          description: "Add an MCP server with a JSON string",
          args: [{ name: "name" }, { name: "json" }],
          options: [clientSecretOption, scopeOption, helpOption],
        },
        {
          name: "get",
          description: "Get details about an MCP server",
          args: mcpServerArg,
          options: [helpOption],
        },
        {
          name: "list",
          description: "List configured MCP servers",
          options: [helpOption],
        },
        {
          name: "login",
          description: "Authenticate with an MCP server",
          args: mcpServerArg,
          options: [
            {
              name: "--no-browser",
              description: "Print the URL instead of opening a browser",
            },
            helpOption,
          ],
        },
        {
          name: "logout",
          description: "Clear stored OAuth credentials",
          args: mcpServerArg,
          options: [helpOption],
        },
        {
          name: "remove",
          description: "Remove an MCP server",
          args: mcpServerArg,
          options: [scopeOption, helpOption],
        },
        {
          name: "reset-project-choices",
          description: "Reset approved and rejected .mcp.json servers",
          options: [helpOption],
        },
        {
          name: "serve",
          description: "Start the Claude Code MCP server",
          options: [
            { name: ["-d", "--debug"], description: "Enable debug mode" },
            { name: "--verbose", description: "Verbose output" },
            helpOption,
          ],
        },
      ],
      options: [helpOption],
    },
    {
      name: ["plugin", "plugins"],
      description: "Manage Claude Code plugins",
      subcommands: [
        {
          name: "configure",
          description: "Show or set a plugin's options",
          args: pluginArg,
          options: [
            jsonOption,
            { name: "--values-stdin", description: "Read values from stdin" },
            helpOption,
          ],
        },
        {
          name: "details",
          description: "Show a plugin's components and token cost",
          args: pluginArg,
          options: [helpOption],
        },
        {
          name: "disable",
          description: "Disable an enabled plugin",
          args: { ...pluginArg, isOptional: true },
          options: [
            { name: ["-a", "--all"], description: "Disable all plugins" },
            jsonOption,
            scopeOption,
            helpOption,
          ],
        },
        {
          name: "enable",
          description: "Enable a disabled plugin",
          args: pluginArg,
          options: [jsonOption, scopeOption, helpOption],
        },
        {
          name: "eval",
          description: "Run a plugin's eval suite",
          args: {
            name: "target",
            isOptional: true,
            template: "folders",
            generators: pluginGenerator,
          },
          options: [helpOption],
        },
        {
          name: ["init", "new"],
          description: "Create a new plugin",
          args: { name: "name" },
          options: [
            {
              name: "--author",
              description: "Author name",
              args: { name: "name" },
            },
            {
              name: "--author-email",
              description: "Author email",
              args: { name: "email" },
            },
            {
              name: "--description",
              description: "Manifest description",
              args: { name: "text" },
            },
            {
              name: ["-f", "--force"],
              description: "Overwrite an existing .claude-plugin/",
            },
            {
              name: "--with",
              description: "Also scaffold these components",
              args: {
                name: "components",
                isVariadic: true,
                suggestions: [
                  "skills",
                  "agents",
                  "hooks",
                  "mcp",
                  "lsp",
                  "output-style",
                  "channel",
                ],
              },
            },
            helpOption,
          ],
        },
        {
          name: ["install", "i"],
          description: "Install a plugin",
          args: { name: "plugin" },
          options: [
            acceptCommandOption,
            {
              name: "--config",
              description: "Set a plugin option",
              args: { name: "key=value" },
            },
            jsonOption,
            {
              name: "--marketplace",
              description: "Marketplace source",
              args: { name: "source" },
            },
            {
              name: "--registry",
              description: "Registry URL",
              args: { name: "url" },
            },
            scopeOption,
            yesOption,
            helpOption,
          ],
        },
        {
          name: "list",
          description: "List installed plugins",
          options: [
            {
              name: "--available",
              description: "Include plugins available from marketplaces",
            },
            {
              name: "--data-size",
              description: "Measure plugin data directories",
              args: { ...pluginArg, isOptional: true },
            },
            jsonOption,
            helpOption,
          ],
        },
        {
          name: "marketplace",
          description: "Manage plugin marketplaces",
          subcommands: [
            {
              name: "add",
              description: "Add a marketplace",
              args: { name: "source" },
            },
            { name: "list", description: "List marketplaces" },
            {
              name: ["remove", "rm"],
              description: "Remove a marketplace",
              args: { name: "name" },
            },
            {
              name: "update",
              description: "Update marketplaces",
              args: { name: "name", isOptional: true },
            },
          ],
          options: [helpOption],
        },
        {
          name: ["prune", "autoremove"],
          description: "Remove unused plugin dependencies",
          options: [helpOption],
        },
        {
          name: "tag",
          description: "Tag a plugin release",
          args: { name: "path", isOptional: true, template: "folders" },
          options: [
            {
              name: "--dry-run",
              description: "Print the tag without creating it",
            },
            {
              name: ["-f", "--force"],
              description: "Skip the dirty-tree and existing-tag checks",
            },
            {
              name: ["-m", "--message"],
              description: "Tag annotation message (%s for the version)",
              args: { name: "msg" },
            },
            { name: "--push", description: "Push the tag after creating it" },
            {
              name: "--remote",
              description: "Remote to push to with --push",
              args: { name: "name" },
            },
            helpOption,
          ],
        },
        {
          name: "test",
          description: "Test a plugin",
          args: { name: "dir", isOptional: true, template: "folders" },
          options: [helpOption],
        },
        {
          name: ["uninstall", "remove"],
          description: "Uninstall a plugin",
          args: pluginArg,
          options: [
            jsonOption,
            {
              name: "--keep-data",
              description: "Keep the plugin's data directory",
            },
            { name: "--prune", description: "Also remove unused dependencies" },
            scopeOption,
            yesOption,
            helpOption,
          ],
        },
        {
          name: "update",
          description: "Update a plugin",
          args: pluginArg,
          options: [
            acceptCommandOption,
            jsonOption,
            scopeOption,
            yesOption,
            helpOption,
          ],
        },
        {
          name: "validate",
          description: "Validate a plugin",
          args: { name: "path", template: ["folders", "filepaths"] },
          options: [
            jsonOption,
            { name: "--strict", description: "Treat warnings as errors" },
            helpOption,
          ],
        },
      ],
      options: [helpOption],
    },
    {
      name: "purge",
      description: "Delete all Claude Code state for a project",
      args: { name: "path", isOptional: true, template: "folders" },
      options: [
        { name: "--all", description: "Purge every project" },
        { name: "--dry-run", description: "Show what would be deleted" },
        { name: ["-i", "--interactive"], description: "Choose what to delete" },
        yesOption,
        helpOption,
      ],
      isDangerous: true,
    },
    {
      name: "respawn",
      description: "Restart a background session on the current version",
      args: { ...backgroundSessionArg, isOptional: true },
      options: [
        { name: "--all", description: "Restart all background sessions" },
        helpOption,
      ],
    },
    {
      name: "rm",
      description: "Delete a background session",
      args: backgroundSessionArg,
      options: [
        {
          name: "--discard-unpushed",
          description: "Discard unpushed commits",
          args: { name: "commit@worktree-id" },
        },
        {
          name: "--force-remove-worktree",
          description: "Remove the worktree anyway",
          args: { name: "worktree-id" },
        },
        helpOption,
      ],
    },
    {
      name: "setup-token",
      description: "Set up a long-lived authentication token",
      options: [helpOption],
    },
    {
      name: ["stop", "kill"],
      description: "Stop a background session",
      args: backgroundSessionArg,
    },
    {
      name: "ultrareview",
      description: "Run a cloud-hosted multi-agent code review",
      args: { name: "target", isOptional: true },
      options: [
        jsonOption,
        { name: "--post", description: "Post findings to the PR" },
        { name: "--no-post", description: "Don't offer to post findings" },
        {
          name: "--timeout",
          description: "Timeout in minutes",
          args: { name: "minutes" },
        },
        helpOption,
      ],
    },
    {
      name: ["update", "upgrade"],
      description: "Check for updates and install them",
      options: [helpOption],
    },
  ],
  options: [
    ...sessionOptions,
    {
      name: "--add-dir",
      description: "Additional directories to allow tool access to",
      args: { name: "directories", template: "folders", isVariadic: true },
    },
    {
      name: "--mcp-config",
      description: "Load MCP servers from JSON files or strings",
      args: { name: "configs", template: "filepaths", isVariadic: true },
    },
    {
      name: "--agents",
      description: "JSON object or file defining custom agents",
      args: { name: "json-or-file" },
    },
    {
      name: ["--allowedTools", "--allowed-tools"],
      description: 'Tools to allow (e.g. "Bash(git *) Edit")',
      args: { name: "tools", isVariadic: true },
    },
    {
      name: "--append-system-prompt",
      description: "Append to the default system prompt",
      args: { name: "prompt" },
    },
    {
      name: "--autocompact",
      description: "Auto-compact window size",
      args: { name: "auto|tokens", suggestions: ["auto"] },
    },
    {
      name: "--ax-screen-reader",
      description: "Screen-reader friendly output",
    },
    {
      name: ["--bg", "--background"],
      description: "Start the session in the background",
    },
    {
      name: "--bare",
      description: "Minimal mode: skip hooks, plugins, CLAUDE.md and more",
    },
    {
      name: "--betas",
      description: "Beta headers for API requests",
      args: { name: "betas", isVariadic: true },
    },
    { name: "--brief", description: "Enable the SendUserMessage tool" },
    { name: "--chrome", description: "Enable Claude in Chrome integration" },
    {
      name: "--no-chrome",
      description: "Disable Claude in Chrome integration",
    },
    {
      name: "--cloud",
      description: "Create or attach to a cloud session",
      args: { name: "description|session_id|url", isOptional: true },
    },
    {
      name: ["-c", "--continue"],
      description: "Continue the most recent conversation in this directory",
    },
    {
      name: ["-d", "--debug"],
      description: "Enable debug mode with optional category filter",
      args: { name: "filter", isOptional: true },
    },
    {
      name: "--debug-file",
      description: "Write debug logs to a file",
      args: { name: "path", template: "filepaths" },
    },
    { name: "--desktop", description: "Open in the Claude Desktop app" },
    { name: "--disable-slash-commands", description: "Disable all skills" },
    {
      name: ["--disallowedTools", "--disallowed-tools"],
      description: 'Tools to deny (e.g. "Bash(git *) Edit")',
      args: { name: "tools", isVariadic: true },
    },
    {
      name: "--environment",
      description: "Self-hosted environment for a cloud session",
      args: { name: "environment_id" },
    },
    {
      name: "--exclude-dynamic-system-prompt-sections",
      description: "Move per-machine sections into the first user message",
    },
    {
      name: "--fallback-model",
      description: "Fallback model(s) when the default is unavailable",
      args: { name: "model", suggestions: models },
    },
    {
      name: "--file",
      description: "File resources to download (file_id:path)",
      args: { name: "specs", isVariadic: true },
    },
    {
      name: "--fork-session",
      description: "Use a new session ID when resuming",
    },
    {
      name: "--forward-subagent-text",
      description: "Forward subagent text with --print stream-json",
    },
    {
      name: "--from-pr",
      description: "Resume a session linked to a PR",
      args: { name: "value", isOptional: true },
    },
    helpOption,
    { name: "--ide", description: "Connect to an IDE on startup" },
    {
      name: "--include-hook-events",
      description: "Include hook events with stream-json",
    },
    {
      name: "--include-partial-messages",
      description: "Include partial message chunks with stream-json",
    },
    {
      name: "--input-format",
      description: "Input format (with --print)",
      args: { name: "format", suggestions: ["text", "stream-json"] },
    },
    {
      name: "--json-schema",
      description: "JSON Schema for structured output",
      args: { name: "schema" },
    },
    {
      name: "--max-budget-usd",
      description: "Maximum spend on API calls (with --print)",
      args: { name: "amount" },
    },
    {
      name: ["-n", "--name"],
      description: "Display name for this session",
      args: { name: "name" },
    },
    {
      name: "--no-session-persistence",
      description: "Don't save the session (with --print)",
    },
    {
      name: "--output-format",
      description: "Output format (with --print)",
      args: { name: "format", suggestions: ["text", "json", "stream-json"] },
    },
    {
      name: "--permission-prompts",
      description: "Who answers permission prompts with --print",
      args: { name: "target", suggestions: ["host", "none"] },
    },
    {
      name: "--plugin-url",
      description: "Fetch a plugin .zip for this session",
      args: { name: "url" },
    },
    { name: ["-p", "--print"], description: "Print response and exit" },
    {
      name: "--prompt-suggestions",
      description: "Enable prompt suggestions",
      args: { name: "value", isOptional: true, suggestions: ["true", "false"] },
    },
    {
      name: "--remote-control",
      description: "Start with Remote Control enabled",
      args: { name: "name", isOptional: true },
    },
    {
      name: "--remote-control-session-name-prefix",
      description: "Prefix for Remote Control session names",
      args: { name: "prefix" },
    },
    {
      name: "--replay-user-messages",
      description: "Re-emit user messages on stdout with stream-json",
    },
    {
      name: ["-r", "--resume"],
      description: "Resume a conversation by session ID",
      args: { name: "session", isOptional: true, generators: sessionGenerator },
    },
    {
      name: "--safe-mode",
      description: "Start with all customizations disabled",
    },
    {
      name: "--session-id",
      description: "Use a specific session ID (UUID)",
      args: { name: "uuid" },
    },
    {
      name: "--system-prompt",
      description: "System prompt for the session",
      args: { name: "prompt" },
    },
    {
      name: "--system-prompt-snapshot",
      description: "Record the system prompt once per conversation",
      args: { name: "on|off", suggestions: ["on", "off"] },
    },
    {
      name: "--teleport",
      description: "Resume a teleport session",
      args: { name: "session", isOptional: true },
    },
    {
      name: "--tmux",
      description: "Create a tmux session for the worktree (with --worktree)",
      requiresEquals: true,
      args: { name: "mode", isOptional: true, suggestions: ["classic"] },
    },
    {
      name: "--tools",
      description: 'Built-in tools to enable ("" for none, "default" for all)',
      args: {
        name: "tools",
        isVariadic: true,
        suggestions: ["default", "Bash", "Edit", "Read"],
      },
    },
    { name: "--verbose", description: "Override verbose mode setting" },
    { name: ["-v", "--version"], description: "Output the version number" },
    {
      name: ["-w", "--worktree"],
      description: "Create a new git worktree for this session",
      args: { name: "name", isOptional: true },
    },
  ],
  args: { name: "prompt", isOptional: true },
};

export default completionSpec;
