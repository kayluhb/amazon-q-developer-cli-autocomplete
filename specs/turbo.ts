// Updated by hand from turbo version 2.11.8 help output (2026-10-13); based on withfig/autocomplete's turbo spec
import { gitGenerators } from "./git";

const turboIcon =
  "data:image/png;base64,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";

interface TurboTask {
  cache?: boolean;
  dependsOn?: string[];
  outputs?: string[];
}

interface TurboJSON {
  $schema?: string;
  globalDependencies?: string[];
  // turbo 2.x
  tasks?: Record<string, TurboTask>;
  // turbo 1.x
  pipeline?: Record<string, TurboTask>;
}

const tasksGenerator: Fig.Generator = {
  script: [
    "bash",
    "-c",
    "until [[ ( -f turbo.json || $PWD = '/' ) ]]; do cd ..; done; cat turbo.json",
  ],
  postProcess: (out) => {
    let turbo: TurboJSON;
    try {
      turbo = JSON.parse(out);
    } catch (err: unknown) {
      console.error(err);
      return [];
    }
    const tasks = turbo.tasks ?? turbo.pipeline ?? {};
    return Object.entries(tasks).map(([key, value]) => {
      const descriptionParts = [];
      if (value.dependsOn?.length) {
        descriptionParts.push(
          `depends on ${value.dependsOn.map((item) => `'${item}'`).join(", ")}`,
        );
      }
      if (value.outputs?.length) {
        descriptionParts.push(
          `outputs ${value.outputs.map((item) => `'${item}'`).join(", ")}`,
        );
      }
      return {
        name: key,
        description:
          descriptionParts.length > 0
            ? "Task: " + descriptionParts.join(", ")
            : "Task",
        icon: turboIcon,
      };
    });
  },
};

const tasksArg: Fig.Arg = {
  name: "tasks",
  isVariadic: true,
  generators: tasksGenerator,
};

const filterOption: Fig.Option = {
  name: ["-F", "--filter"],
  description:
    "Use the given selector to specify package(s) to act as entry points (pnpm filter syntax)",
  isRepeatable: true,
  args: { name: "filter" },
};

const helpOption: Fig.Option = {
  name: ["-h", "--help"],
  description: "Print help",
  isPersistent: true,
};

// Options shared by `turbo run` and `turbo watch`
const executionOptions: Fig.Option[] = [
  {
    name: "--cache-dir",
    description: "Override the filesystem cache directory",
    args: { name: "dir", template: "folders" },
  },
  {
    name: "--concurrency",
    description:
      "Limit the concurrency of task execution. Use 1 for serial execution",
    args: { name: "limit" },
  },
  {
    name: "--continue",
    description:
      "Specify how task execution should proceed when an error occurs",
    requiresSeparator: true,
    args: {
      name: "mode",
      isOptional: true,
      suggestions: [
        { name: "never", description: "Cancel all tasks (default)" },
        {
          name: "dependencies-successful",
          description: "Continue running tasks whose dependencies succeeded",
        },
        {
          name: "always",
          description:
            "Continue running all tasks, even if dependencies failed",
        },
      ],
    },
  },
  {
    name: "--single-package",
    description: "Run turbo in single-package mode",
  },
  {
    name: "--framework-inference",
    description:
      "Specify whether or not to do framework inference for tasks (default: true)",
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--global-deps",
    description:
      "Specify glob of global filesystem dependencies to be hashed. Useful for .env and files",
    isRepeatable: true,
    insertValue: "--global-deps='{cursor}'",
    args: { name: "glob" },
  },
  {
    name: "--env-mode",
    description: "Environment variable mode",
    args: {
      name: "mode",
      isOptional: true,
      suggestions: [
        { name: "strict", description: "Use the allowlist in turbo.json" },
        { name: "loose", description: "Pass the entire existing environment" },
      ],
    },
  },
  filterOption,
  {
    name: "--affected",
    description:
      "Filter to only packages affected by changes between the current branch and `main`",
  },
  {
    name: "--output-logs",
    description: "Set type of process output logging",
    args: {
      name: "level",
      suggestions: [
        { name: "full", description: "Show all output (default)" },
        { name: "hash-only", description: "Only turbo-computed task hashes" },
        {
          name: "new-only",
          description: "Only new output with hashes for cached tasks",
        },
        { name: "errors-only", description: "Only output from failed tasks" },
        { name: "none", description: "Hide process output" },
      ],
    },
  },
  {
    name: "--log-order",
    description: "Set type of task output order",
    args: {
      name: "order",
      suggestions: [
        { name: "auto", description: "Let turbo decide (default)" },
        { name: "stream", description: "Show output as soon as available" },
        { name: "grouped", description: "Show output when a task finishes" },
      ],
    },
  },
  {
    name: "--json",
    description:
      "Output machine-readable NDJSON to stdout. Disables the TUI and forces stream mode",
  },
  {
    name: "--log-file",
    description:
      "Write structured JSON logs to a file (default: .turbo/logs/<epoch_millis>.json)",
    requiresSeparator: true,
    args: { name: "file", isOptional: true, template: "filepaths" },
  },
  {
    name: "--only",
    description: "Only execute the tasks specified, not their dependency tasks",
  },
  {
    name: "--log-prefix",
    description: "Set how task logs are prefixed",
    args: {
      name: "prefix",
      suggestions: [
        { name: "auto", description: "Let turbo decide (default)" },
        { name: "task", description: "Prefix with the task id" },
        { name: "none", description: "No prefix" },
      ],
    },
  },
];

const runOptions: Fig.Option[] = [
  {
    name: "--cache",
    description:
      "Set the cache behavior for this run, e.g. `local:rw,remote:r`",
    args: {
      name: "behavior",
      suggestions: [
        "local:rw,remote:rw",
        "local:rw",
        "remote:rw",
        "local:r,remote:r",
      ],
    },
  },
  {
    name: "--force",
    description:
      "Ignore the existing cache (to force execution). Equivalent to `--cache=local:w,remote:w`",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--remote-only",
    description:
      "Ignore the local filesystem cache; only use the remote cache. Equivalent to `--cache=remote:rw`",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--remote-cache-read-only",
    description:
      "Treat remote cache as read only. Equivalent to `--cache=remote:r;local:rw`",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--no-cache",
    description:
      "Avoid saving task results to the cache. Useful for development/watch tasks",
  },
  {
    name: "--cache-workers",
    description: "Set the number of concurrent cache operations (default: 10)",
    args: { name: "count" },
  },
  {
    name: ["--dry-run", "--dry"],
    description: "List the packages in scope and the tasks that would be run",
    requiresSeparator: true,
    args: {
      name: "format",
      isOptional: true,
      suggestions: ["text", "json"],
    },
  },
  {
    name: "--graph",
    description:
      "Generate a graph of the task execution (.svg, .html, .mermaid, .dot); prints dot to stdout without a file",
    requiresSeparator: true,
    args: { name: "file", isOptional: true, template: "filepaths" },
  },
  {
    name: "--daemon",
    description: "[DEPRECATED] The daemon is no longer used for `turbo run`",
    hidden: true,
  },
  {
    name: "--no-daemon",
    description: "[DEPRECATED] The daemon is no longer used for `turbo run`",
    hidden: true,
  },
  {
    name: "--profile",
    description:
      "File to write turbo's performance profile output into (load in chrome://tracing)",
    requiresSeparator: true,
    args: { name: "file", isOptional: true, template: "filepaths" },
  },
  {
    name: "--anon-profile",
    description:
      "File to write turbo's performance profile into, with identifying data omitted",
    requiresSeparator: true,
    args: { name: "file", isOptional: true, template: "filepaths" },
  },
  {
    name: "--summarize",
    description: "Generate a summary of the turbo run",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--parallel",
    description:
      "[DEPRECATED] Execute all tasks in parallel. Use task configuration instead",
    deprecated: true,
  },
  ...executionOptions,
];

const globalOptionList: Fig.Option[] = [
  {
    name: "--skip-infer",
    description:
      "Skip any attempts to infer which version of Turbo the project is configured to use",
  },
  {
    name: "--no-update-notifier",
    description: "Disable the turbo update notification",
  },
  {
    name: "--api",
    description: "Override the endpoint for API calls",
    args: { name: "url" },
  },
  {
    name: "--color",
    description: "Force color usage in the terminal",
  },
  {
    name: "--no-color",
    description: "Suppress color usage in the terminal",
  },
  {
    name: "--cwd",
    description: "The directory in which to run turbo",
    args: { name: "dir", template: "folders" },
  },
  {
    name: "--heap",
    description: "Specify a file to save a pprof heap profile",
    args: { name: "file", template: "filepaths" },
  },
  {
    name: "--ui",
    description: "Specify whether to use the streaming UI or TUI",
    args: {
      name: "ui",
      suggestions: [
        { name: "tui", description: "Interactive terminal UI" },
        { name: "stream", description: "Streaming logs" },
      ],
    },
  },
  {
    name: "--login",
    description: "Override the login endpoint",
    args: { name: "url" },
  },
  {
    name: "--preflight",
    description:
      "Precede HTTP requests with an OPTIONS request for authorization",
  },
  {
    name: "--remote-cache-timeout",
    description: "Set a timeout for all HTTP requests",
    args: { name: "seconds" },
  },
  {
    name: "--team",
    description: "Set the team slug for API calls",
    args: { name: "slug" },
  },
  {
    name: "--token",
    description: "Set the auth token for API calls",
    args: { name: "token" },
  },
  {
    name: "--trace",
    description: "Specify a file to save a pprof trace",
    args: { name: "file", template: "filepaths" },
  },
  {
    name: "--verbosity",
    description:
      "Verbosity level. Useful when debugging Turborepo or creating logs for issue reports",
    args: { name: "count" },
  },
  {
    name: "--experimental-otel-enabled",
    description: "Enable OpenTelemetry metrics export",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--experimental-otel-protocol",
    description: "OTLP transport protocol",
    args: { name: "protocol", suggestions: ["grpc", "http-protobuf"] },
  },
  {
    name: "--experimental-otel-endpoint",
    description: "OTLP collector endpoint URL",
    args: { name: "url" },
  },
  {
    name: "--experimental-otel-timeout-ms",
    description: "OTLP export timeout in milliseconds (default: 10000)",
    args: { name: "milliseconds" },
  },
  {
    name: "--experimental-otel-interval-ms",
    description: "OTLP export interval in milliseconds (default: 15000)",
    args: { name: "milliseconds" },
  },
  {
    name: "--experimental-otel-header",
    description: "Add header to OTLP requests",
    isRepeatable: true,
    args: { name: "key=value" },
  },
  {
    name: "--experimental-otel-resource",
    description: "Add resource attribute to metrics",
    isRepeatable: true,
    args: { name: "key=value" },
  },
  {
    name: "--experimental-otel-metrics-run-summary",
    description: "Emit run-level summary metrics (default: true)",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--experimental-otel-metrics-task-details",
    description: "Emit per-task detail metrics (default: false)",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--experimental-otel-use-remote-cache-token",
    description: "Use remote cache token for OTLP authentication",
    requiresSeparator: true,
    args: { name: "bool", isOptional: true, suggestions: ["true", "false"] },
  },
  {
    name: "--dangerously-disable-package-manager-check",
    description: "Allow for missing `packageManager` in `package.json`",
  },
  {
    name: "--root-turbo-json",
    description:
      "Use the turbo.json at the provided path instead of the one at the repository root",
    args: { name: "path", template: "filepaths" },
  },
];

const globalOptions: Fig.Option[] = globalOptionList.map((option) => ({
  ...option,
  isPersistent: true,
}));

const generatorOptions: Fig.Option[] = [
  {
    name: ["-c", "--config"],
    description: "Generator configuration file",
    args: { name: "config", template: "filepaths" },
  },
  {
    name: ["-r", "--root"],
    description:
      "The root of your repository (default: directory with root turbo.json)",
    args: { name: "root", template: "folders" },
  },
  {
    name: ["-a", "--args"],
    description: "Answers passed directly to generator",
    args: { name: "args", isVariadic: true },
  },
];

const completionSpec: Fig.Spec = {
  name: "turbo",
  description: "The build system that makes ship happen",
  icon: turboIcon,
  // `turbo <tasks>` is shorthand for `turbo run <tasks>`
  args: { ...tasksArg, isOptional: true },
  options: [
    {
      name: "--version",
      description: "Print the version",
    },
    helpOption,
    ...globalOptions,
    ...runOptions,
  ],
  subcommands: [
    {
      name: "bin",
      description: "Get the path to the Turbo binary",
    },
    {
      name: "completion",
      description: "Generate the autocompletion script for the specified shell",
      args: {
        name: "shell",
        suggestions: ["bash", "elvish", "fish", "powershell", "zsh"],
      },
    },
    {
      name: "daemon",
      description: "Runs the Turborepo background daemon",
      options: [
        {
          name: "--idle-time",
          description: "Set the idle timeout for turbod (default: 4h0m0s)",
          args: { name: "duration" },
        },
        {
          name: "--turbo-json-path",
          description: "Path to a custom turbo.json file to watch",
          args: { name: "path", template: "filepaths" },
        },
      ],
      subcommands: [
        {
          name: "clean",
          description:
            "Stops the turbo daemon if it is already running, and removes any stale daemon state",
          options: [
            {
              name: "--clean-logs",
              description: "Clean the daemon logs too (default: true)",
            },
          ],
        },
        { name: "logs", description: "Shows the daemon logs" },
        { name: "restart", description: "Restarts the turbo daemon" },
        {
          name: "start",
          description: "Ensures that the turbo daemon is running",
        },
        {
          name: "status",
          description: "Reports the status of the turbo daemon",
          options: [
            { name: "--json", description: "Report status in JSON format" },
          ],
        },
        { name: "stop", description: "Stops the turbo daemon" },
      ],
    },
    {
      name: "devtools",
      description: "Visualize your monorepo's package graph in the browser",
      options: [
        {
          name: "--port",
          description: "Port for the WebSocket server (default: 9876)",
          args: { name: "port" },
        },
        {
          name: "--no-open",
          description: "Don't automatically open the browser",
        },
      ],
    },
    {
      name: "docs",
      description: "Search the Turborepo documentation",
      args: { name: "query", description: "The search query" },
      options: [
        {
          name: "--docs-version",
          description: "Override the docs version (minimum: 2.7.5)",
          args: { name: "version" },
        },
      ],
    },
    {
      name: ["generate", "g", "gen"],
      description: "Generate a new app / package",
      args: {
        name: "generator",
        description: "The name of the generator to run",
        isOptional: true,
      },
      options: generatorOptions,
      subcommands: [
        {
          name: ["run", "r"],
          description: "Run a custom generator",
          args: {
            name: "generator",
            description: "The name of the generator to run",
            isOptional: true,
          },
          options: generatorOptions,
        },
        {
          name: ["workspace", "w"],
          description: "Add a new package or app to your project",
          options: [
            {
              name: ["-n", "--name"],
              description: "Name for the new workspace",
              args: { name: "name" },
            },
            {
              name: ["-b", "--empty"],
              description: "Generate an empty workspace",
            },
            {
              name: ["-c", "--copy"],
              description:
                "Generate a workspace using an existing workspace or a GitHub URL as a template",
              args: { name: "source", isOptional: true },
            },
            {
              name: ["-d", "--destination"],
              description: "Where the new workspace should be created",
              args: { name: "destination", template: "folders" },
            },
            {
              name: ["-t", "--type"],
              description: "The type of workspace to create",
              args: { name: "type", suggestions: ["app", "package"] },
            },
            {
              name: ["-r", "--root"],
              description:
                "The root of your repository (default: directory with root turbo.json)",
              args: { name: "root", template: "folders" },
            },
            {
              name: ["-p", "--example-path"],
              description:
                "Path to the example when the GitHub URL's branch name contains a slash",
              args: { name: "path" },
            },
            {
              name: "--show-all-dependencies",
              description:
                "Do not filter available dependencies by the workspace type",
            },
          ],
        },
      ],
    },
    {
      name: "get-mfe-port",
      description: "Get the port assigned to the current microfrontend",
    },
    {
      name: "info",
      description: "Print debugging information",
    },
    {
      name: "link",
      description:
        "Link your local directory to a Vercel organization and enable remote caching",
      options: [
        {
          name: "--no-gitignore",
          description: "Do not create or modify .gitignore",
        },
        {
          name: "--scope",
          description: "The scope, i.e. Vercel team, to which you are linking",
          args: { name: "scope" },
        },
        {
          name: ["-y", "--yes"],
          description: "Answer yes to all prompts",
        },
      ],
    },
    {
      name: "login",
      description: "Login to your Vercel account",
      options: [
        {
          name: "--sso-team",
          description:
            "Attempt to authenticate to the specified team using SSO",
          args: { name: "team" },
        },
        {
          name: "--manual",
          description:
            "Manually enter token instead of requesting one from the login service",
        },
      ],
    },
    {
      name: "logout",
      description: "Logout of your Vercel account",
      options: [
        {
          name: "--invalidate",
          description:
            "Invalidate the token on the server (default: true). Pass `--invalidate=false` to skip",
          requiresSeparator: true,
          args: {
            name: "bool",
            isOptional: true,
            suggestions: ["true", "false"],
          },
        },
      ],
    },
    {
      name: "ls",
      description: "List packages in your monorepo",
      args: {
        name: "packages",
        description: "Get insight into specific packages",
        isVariadic: true,
        isOptional: true,
      },
      options: [
        {
          name: "--affected",
          description:
            "Show only packages affected by changes between the current branch and `main`",
        },
        filterOption,
        {
          name: "--output",
          description: "EXPERIMENTAL: Output format",
          args: { name: "format", suggestions: ["pretty", "json"] },
        },
      ],
    },
    {
      name: "prune",
      description: "Prepare a subset of your monorepo",
      args: {
        name: "scope",
        description: "Workspaces that should be included in the subset",
        isVariadic: true,
        isOptional: true,
      },
      options: [
        {
          name: "--docker",
          description:
            "Output pruned workspace into 'full' and 'json' directories optimized for Docker layer caching",
        },
        {
          name: "--production",
          description:
            "Exclude in-workspace devDependencies when selecting packages to include",
        },
        {
          name: "--out-dir",
          description: "Output directory (default: out)",
          args: { name: "dir", template: "folders" },
        },
        {
          name: "--use-gitignore",
          description:
            "Respect `.gitignore` when copying files to the output directory",
          requiresSeparator: true,
          args: {
            name: "bool",
            isOptional: true,
            suggestions: ["true", "false"],
          },
        },
      ],
    },
    {
      name: "query",
      description:
        "Query your monorepo using GraphQL. Without a query, starts a GraphQL server with GraphiQL",
      args: {
        name: "query",
        description: "The query to run, either a file path or a query string",
        isOptional: true,
        template: "filepaths",
      },
      options: [
        {
          name: ["-V", "--variables"],
          description: "Pass variables to the query via a JSON file",
          args: { name: "file", template: "filepaths" },
        },
        {
          name: "--schema",
          description: "Print the GraphQL schema",
        },
      ],
      subcommands: [
        {
          name: "affected",
          description:
            "Check which packages or tasks are affected by changes between two git refs",
          options: [
            {
              name: "--packages",
              description:
                "Return affected packages instead of tasks. Optionally filter by name",
              args: { name: "packages", isVariadic: true, isOptional: true },
            },
            {
              name: "--tasks",
              description: "Filter to specific task names (e.g. build, test)",
              args: {
                name: "tasks",
                isVariadic: true,
                isOptional: true,
                generators: tasksGenerator,
              },
            },
            {
              name: "--base",
              description: "Base git ref for comparison",
              args: { name: "ref", generators: gitGenerators.revs },
            },
            {
              name: "--head",
              description: "Head git ref for comparison",
              args: { name: "ref", generators: gitGenerators.revs },
            },
            {
              name: "--exit-code",
              description:
                "Exit with 1 when anything is affected, 0 when nothing is, or 2 on errors",
            },
          ],
        },
        {
          name: "ls",
          description:
            "List packages in your monorepo (shorthand for a packages query)",
          args: {
            name: "packages",
            description: "Get insight into specific packages",
            isVariadic: true,
            isOptional: true,
          },
          options: [
            {
              name: "--affected",
              description:
                "Show only packages affected by changes between the current branch and `main`",
            },
            filterOption,
            {
              name: "--output",
              description: "Output format",
              args: { name: "format", suggestions: ["pretty", "json"] },
            },
          ],
        },
      ],
    },
    {
      name: "run",
      description: "Run tasks across projects in your monorepo",
      icon: turboIcon,
      args: tasksArg,
      options: runOptions,
    },
    {
      name: "setup",
      description:
        "EXPERIMENTAL: Provision repository tools and dependencies (not yet implemented)",
      options: [
        {
          name: "--plan",
          description:
            "Report the proposed setup without writing or installing",
        },
        {
          name: "--check",
          description:
            "Check installed state without writing, resolving, or downloading",
        },
        {
          name: "--force",
          description:
            "Reinstall selected versions without refreshing resolution",
        },
        {
          name: "--frozen",
          description:
            "Require an existing, current turbo.lock (the default in CI)",
        },
        {
          name: "--no-frozen",
          description: "Allow lock creation or updates, including in CI",
        },
        {
          name: "--offline",
          description: "Use cached artifacts only; never make HTTP requests",
        },
        {
          name: "--tools-only",
          description: "Skip dependency installation",
        },
        {
          name: "--no-lock",
          description:
            "Provision without writing turbo.lock; overrides CI-inferred frozen mode",
        },
        {
          name: "--update-lock",
          description:
            "Refresh floating requests and provision repository-policy-selected tools",
        },
      ],
    },
    {
      name: "telemetry",
      description: "Enable or disable anonymous telemetry",
      subcommands: [
        { name: "disable", description: "Disables anonymous telemetry" },
        { name: "enable", description: "Enables anonymous telemetry" },
        { name: "status", description: "Reports the status of telemetry" },
      ],
    },
    {
      name: "unlink",
      description:
        "Unlink the current directory from your Vercel organization and disable Remote Caching",
    },
    {
      name: "watch",
      description: "Re-run tasks when files change",
      icon: turboIcon,
      args: tasksArg,
      options: [
        ...executionOptions,
        {
          name: "--experimental-write-cache",
          description: "EXPERIMENTAL: Write to cache in watch mode",
        },
      ],
    },
  ],
};

export default completionSpec;
