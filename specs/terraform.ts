// Updated by hand from terraform v1.16.5 help output (2026-10-13); based on withfig's spec
// If you edit commands or options, please copy our changes on to the terragrunt spec
const workspaceList: Fig.Generator = {
  script: ["terraform", "workspace", "list"],
  postProcess: function (out) {
    return out.split("\n").map((workspace) => {
      return {
        name: workspace.replace("* ", "").trim(),
        icon: "https://www.terraform.io/favicon.ico",
        description: "Workspace",
      };
    });
  },
};

const addressList: Fig.Generator = {
  script: ["terraform", "state", "list"],
  postProcess: function (out) {
    if (out.includes("No state file was found!") || out.includes("Error")) {
      return [];
    }
    return out.split("\n").map((addresses) => {
      return {
        name: addresses.replace("* ", "").trim(),
        icon: "https://www.terraform.io/favicon.ico",
        description: "Address",
      };
    });
  },
};

const booleanArg = (name: string): Fig.Arg => ({
  name,
  suggestions: ["true", "false"],
});

const helpOption: Fig.Option = {
  name: "-help",
  description: "Show the help for this subcommand",
};

const noColorOption: Fig.Option = {
  name: "-no-color",
  description: "If specified, output won't contain any color",
};

const jsonOption = (description: string): Fig.Option => ({
  name: "-json",
  description,
});

const varOptions: Fig.Option[] = [
  {
    name: "-var",
    insertValue: "-var {cursor}",
    description:
      "Set a value for one of the input variables in the root module of the configuration. Use this option more than once to set more than one variable",
    isRepeatable: true,
    args: {
      name: "foo=bar",
    },
  },
  {
    name: "-var-file",
    requiresSeparator: true,
    description:
      "Load variable values from the given file, in addition to the default files terraform.tfvars and *.auto.tfvars. Use this option more than once to include more than one variables file",
    isRepeatable: true,
    args: {
      name: "filename",
      template: "filepaths",
    },
  },
];

const lockOptions: Fig.Option[] = [
  {
    name: "-lock",
    requiresSeparator: true,
    description:
      "Don't hold a state lock during the operation if set to false. This is dangerous if others might concurrently run commands against the same workspace. Defaults to true",
    args: booleanArg("true or false"),
  },
  {
    name: "-lock-timeout",
    requiresSeparator: true,
    description: "Duration to retry a state lock. Defaults to 0s",
    args: {
      name: "duration",
    },
  },
];

const inputOption: Fig.Option = {
  name: "-input",
  requiresSeparator: true,
  description: "Ask for input for variables if not directly set",
  args: booleanArg("true or false"),
};

const compactWarningsOption: Fig.Option = {
  name: "-compact-warnings",
  description:
    "If Terraform produces any warnings that are not accompanied by errors, show them in a more compact form that includes only the summary messages",
};

const parallelismOption: Fig.Option = {
  name: "-parallelism",
  requiresSeparator: true,
  description: "Limit the number of concurrent operations. Defaults to 10",
  args: {
    name: "n",
  },
};

const ignoreRemoteVersionOption: Fig.Option = {
  name: "-ignore-remote-version",
  description:
    "A rare option used for HCP Terraform and the remote backend only. Continue even if remote and local Terraform versions are incompatible",
};

const testDirectoryOption: Fig.Option = {
  name: "-test-directory",
  requiresSeparator: true,
  description: 'Set the Terraform test directory, defaults to "tests"',
  args: {
    name: "path",
    template: "folders",
  },
};

const generateConfigOutOption = (description: string): Fig.Option => ({
  name: "-generate-config-out",
  requiresSeparator: true,
  description,
  args: {
    name: "path",
    template: "filepaths",
  },
});

const stateOption = (description: string): Fig.Option => ({
  name: "-state",
  requiresSeparator: true,
  description,
  args: {
    name: "path",
    template: "filepaths",
  },
});

const legacyStateOptions: Fig.Option[] = [
  stateOption(
    "Path to read and save state. Legacy option for the local backend only",
  ),
  {
    name: "-state-out",
    requiresSeparator: true,
    description:
      "Path to write state to that is different than -state. Legacy option for the local backend only",
    args: {
      name: "path",
      template: "filepaths",
    },
  },
  {
    name: "-backup",
    requiresSeparator: true,
    description:
      'Path to backup the existing state file before modifying. Set to "-" to disable backup. Legacy option for the local backend only',
    args: {
      name: "path",
      template: "filepaths",
    },
  },
];

const targetOption: Fig.Option = {
  name: "-target",
  requiresSeparator: true,
  description:
    "Limit the operation to only the given module, resource, or resource instance and all of its dependencies. Can be used multiple times",
  isRepeatable: true,
  args: {
    name: "resource",
    generators: addressList,
  },
};

const replaceOption: Fig.Option = {
  name: "-replace",
  requiresSeparator: true,
  description:
    "Force replacement of a particular resource instance using its resource address. Can be used multiple times",
  isRepeatable: true,
  args: {
    name: "resource",
    generators: addressList,
  },
};

const destroyModeOption: Fig.Option = {
  name: "-destroy",
  description:
    'Select the "destroy" planning mode, which creates a plan to destroy all objects currently managed by this Terraform configuration',
};

const refreshOnlyOption: Fig.Option = {
  name: "-refresh-only",
  description:
    'Select the "refresh only" planning mode, which checks whether remote objects still match the most recent apply without proposing actions to undo changes made outside of Terraform',
};

const refreshOption: Fig.Option = {
  name: "-refresh",
  requiresSeparator: true,
  description:
    "Check for external changes to remote objects while creating the plan. Defaults to true",
  args: booleanArg("true or false"),
};

// Options shared by plan, apply (without a saved plan) and destroy
const planCustomizationOptions: Fig.Option[] = [
  refreshOption,
  replaceOption,
  targetOption,
  ...varOptions,
];

const applyOptions: Fig.Option[] = [
  {
    name: "-auto-approve",
    description: "Skip interactive approval of plan before applying",
  },
  compactWarningsOption,
  ...lockOptions,
  inputOption,
  noColorOption,
  parallelismOption,
  ...legacyStateOptions,
  ...planCustomizationOptions,
  helpOption,
];

const globalOptions: Fig.Option[] = [
  {
    name: "-help",
    description: "Show this help output or the help for a specified subcommand",
  },
  {
    name: "-chdir",
    description:
      "Switch to a different working directory before executing the given subcommand",
    requiresSeparator: true,
    args: {
      name: "DIR",
      template: "folders",
    },
  },
  {
    name: "-version",
    description: 'An alias for the "version" subcommand',
  },
];

const mainCommands: Fig.Subcommand[] = [
  {
    name: "init",
    description: "Prepare your working directory for other commands",
    options: [
      {
        name: "-backend",
        requiresSeparator: true,
        description:
          "Disable backend or HCP Terraform initialization for this configuration and use what was previously initialized instead",
        args: booleanArg("true or false"),
      },
      {
        name: "-cloud",
        requiresSeparator: true,
        description: "Alias for -backend",
        args: booleanArg("true or false"),
      },
      {
        name: "-backend-config",
        requiresSeparator: true,
        description:
          "Configuration to be merged with what is in the configuration file's 'backend' block. Either a path to an HCL file or a 'key=value' pair; can be specified multiple times",
        isRepeatable: true,
        args: {
          name: "path or key=value",
          template: "filepaths",
        },
      },
      {
        name: "-force-copy",
        description:
          "Suppress prompts about copying state data when initializing a new state backend",
      },
      {
        name: "-from-module",
        requiresSeparator: true,
        description:
          "Copy the contents of the given module into the target directory before initialization",
        args: {
          name: "SOURCE",
        },
      },
      {
        name: "-get",
        requiresSeparator: true,
        description:
          "Download modules for this configuration. Defaults to true",
        args: booleanArg("true or false"),
      },
      inputOption,
      ...lockOptions,
      noColorOption,
      jsonOption(
        "If specified, machine readable output will be printed in JSON format",
      ),
      {
        name: "-plugin-dir",
        requiresSeparator: true,
        description:
          "Directory containing plugin binaries. Overrides all default search paths for plugins and prevents automatic installation. Can be used multiple times",
        isRepeatable: true,
        args: {
          name: "dir",
          template: "folders",
        },
      },
      {
        name: "-reconfigure",
        description: "Reconfigure a backend, ignoring any saved configuration",
      },
      {
        name: "-migrate-state",
        description:
          "Reconfigure a backend, and attempt to migrate any existing state",
      },
      {
        name: "-upgrade",
        description:
          "Install the latest module and provider versions allowed within configured constraints, overriding the dependency lockfile",
      },
      {
        name: "-lockfile",
        requiresSeparator: true,
        description: "Set a dependency lockfile mode",
        args: {
          name: "MODE",
          suggestions: ["readonly"],
        },
      },
      ignoreRemoteVersionOption,
      testDirectoryOption,
      ...varOptions,
      {
        name: "-enable-pluggable-state-storage-experiment",
        description:
          "[EXPERIMENTAL] Enable an alternative init command that allows use of pluggable state storage",
      },
      {
        name: "-state-provider-lock-file",
        requiresSeparator: true,
        description:
          "[EXPERIMENTAL] Lock file used to establish trust in a provider before initializing a state store for the first time",
        args: {
          name: "path",
          template: "filepaths",
        },
      },
      helpOption,
    ],
  },
  {
    name: "validate",
    description: "Check whether the configuration is valid",
    options: [
      jsonOption("Produce output in a machine-readable JSON format"),
      noColorOption,
      {
        name: "-no-tests",
        description: "If specified, Terraform will not validate test files",
      },
      testDirectoryOption,
      {
        name: "-query",
        description:
          "If specified, the command will also validate .tfquery.hcl files",
      },
      ...varOptions,
      helpOption,
    ],
  },
  {
    name: "plan",
    description: "Show changes required by the current configuration",
    options: [
      destroyModeOption,
      refreshOnlyOption,
      ...planCustomizationOptions,
      compactWarningsOption,
      {
        name: "-detailed-exitcode",
        description:
          "Return detailed exit codes: 0 - no changes, 1 - errored, 2 - succeeded with a diff",
      },
      generateConfigOutOption(
        "(Experimental) If import blocks are present in configuration, generate HCL for any imported resources not already present, written to a new file at PATH",
      ),
      inputOption,
      ...lockOptions,
      noColorOption,
      {
        name: "-out",
        requiresSeparator: true,
        description:
          'Write a plan file to the given path. This can be used as input to the "apply" command',
        args: {
          name: "path",
          template: "filepaths",
        },
      },
      parallelismOption,
      stateOption("A legacy option used for the local backend only"),
      helpOption,
    ],
  },
  {
    name: "apply",
    description: "Create or update infrastructure",
    args: {
      name: "PLAN",
      isOptional: true,
      template: "filepaths",
    },
    options: [
      {
        ...destroyModeOption,
        description:
          'Destroy Terraform-managed infrastructure. "terraform destroy" is a convenience alias for this option',
      },
      refreshOnlyOption,
      ...applyOptions,
    ],
  },
  {
    name: "destroy",
    description: "Destroy previously-created infrastructure",
    options: applyOptions,
  },
];

const otherCommands: Fig.Subcommand[] = [
  {
    name: "console",
    description: "Try Terraform expressions at an interactive command prompt",
    options: [
      stateOption("Legacy option for the local backend only"),
      {
        name: "-plan",
        description:
          "Create a new plan and evaluate expressions against its planned state, instead of against the current state",
      },
      {
        name: "-scope",
        requiresSeparator: true,
        description:
          "A module instance address which declares the scope to use when evaluating expressions. Defaults to the root module",
        args: {
          name: "module",
        },
      },
      ...varOptions,
      helpOption,
    ],
  },
  {
    name: "fmt",
    description: "Reformat your configuration in the standard style",
    args: {
      name: "target",
      isOptional: true,
      isVariadic: true,
      template: "filepaths",
    },
    options: [
      {
        name: "-list",
        requiresSeparator: true,
        description: "List files whose formatting differs. Defaults to true",
        args: booleanArg("true or false"),
      },
      {
        name: "-write",
        requiresSeparator: true,
        description: "Write to source files. Defaults to true",
        args: booleanArg("true or false"),
      },
      {
        name: "-diff",
        description: "Display diffs of formatting changes",
      },
      {
        name: "-check",
        description:
          "Check if the input is formatted. Exit status will be 0 if all input is properly formatted and non-zero otherwise",
      },
      noColorOption,
      {
        name: "-recursive",
        description: "Also process files in subdirectories",
      },
      helpOption,
    ],
  },
  {
    name: "force-unlock",
    description: "Release a stuck lock on the current workspace",
    args: {
      name: "LOCK_ID",
    },
    options: [
      {
        name: "-force",
        description: "Don't ask for input for unlock confirmation",
      },
      helpOption,
    ],
  },
  {
    name: "get",
    description: "Install or upgrade remote Terraform modules",
    options: [
      {
        name: "-update",
        description:
          "Check already-downloaded modules for available updates and install the newest versions available",
      },
      noColorOption,
      testDirectoryOption,
      ...varOptions,
      helpOption,
    ],
  },
  {
    name: "graph",
    description: "Generate a Graphviz graph of the steps in an operation",
    options: [
      {
        name: "-plan",
        requiresSeparator: true,
        description:
          "Render graph using the specified plan file instead of the configuration in the current directory. Implies -type=apply",
        args: {
          name: "tfplan",
          template: "filepaths",
        },
      },
      {
        name: "-draw-cycles",
        description: "Highlight any cycles in the graph with colored edges",
      },
      {
        name: "-type",
        requiresSeparator: true,
        description: "Type of operation graph to output",
        args: {
          name: "TYPE",
          suggestions: ["plan", "plan-refresh-only", "plan-destroy", "apply"],
        },
      },
      {
        name: "-module-depth",
        requiresSeparator: true,
        description: "(deprecated) Has no effect",
        hidden: true,
        deprecated: true,
        args: {
          name: "n",
        },
      },
      ...varOptions,
      {
        name: "-format",
        requiresSeparator: true,
        description: "Output format for the graph. Defaults to dot",
        args: {
          name: "FORMAT",
          suggestions: ["dot", "mermaid"],
        },
      },
      helpOption,
    ],
  },
  {
    name: "import",
    description: "Associate existing infrastructure with a Terraform resource",
    args: [
      {
        name: "ADDR",
      },
      {
        name: "ID",
      },
    ],
    options: [
      {
        name: "-config",
        requiresSeparator: true,
        description:
          "Path to a directory of Terraform configuration files to use to configure the provider. Defaults to pwd",
        args: {
          name: "path",
          template: "folders",
        },
      },
      inputOption,
      ...lockOptions,
      noColorOption,
      ...varOptions,
      ignoreRemoteVersionOption,
      ...legacyStateOptions,
      helpOption,
    ],
  },
  {
    name: "login",
    description: "Obtain and save credentials for a remote host",
    args: {
      name: "hostname",
      isOptional: true,
    },
    options: [helpOption],
  },
  {
    name: "logout",
    description: "Remove locally-stored credentials for a remote host",
    args: {
      name: "hostname",
      isOptional: true,
    },
    options: [helpOption],
  },
  {
    name: "metadata",
    description: "Metadata related commands",
    options: [helpOption],
    subcommands: [
      {
        name: "functions",
        description:
          "Show signatures and descriptions for the available functions",
        options: [
          jsonOption(
            "Print a JSON representation of the available function signatures",
          ),
          helpOption,
        ],
      },
    ],
  },
  {
    name: "modules",
    description: "Show all declared modules in a working directory",
    options: [
      jsonOption(
        "Output declared Terraform modules and their resolved versions in a machine-readable format",
      ),
      ...varOptions,
      helpOption,
    ],
  },
  {
    name: "output",
    description: "Show output values from your root module",
    args: {
      name: "NAME",
      isOptional: true,
    },
    options: [
      stateOption(
        'Path to the state file to read. Defaults to "terraform.tfstate". Legacy option for the local backend only',
      ),
      noColorOption,
      jsonOption(
        "If specified, machine readable output will be printed in JSON format",
      ),
      {
        name: "-raw",
        description:
          "For value types that can be automatically converted to a string, print the raw string directly",
      },
      helpOption,
    ],
  },
  {
    name: "providers",
    description: "Show the providers required for this configuration",
    args: {
      name: "DIR",
      isOptional: true,
      template: "folders",
    },
    options: [testDirectoryOption, ...varOptions, helpOption],
    subcommands: [
      {
        name: "lock",
        description: "Write out dependency locks for the configured providers",
        args: {
          name: "providers",
          isOptional: true,
          isVariadic: true,
        },
        options: [
          {
            name: "-fs-mirror",
            requiresSeparator: true,
            description:
              "Consult the given filesystem mirror directory instead of the origin registry",
            args: {
              name: "dir",
              template: "folders",
            },
          },
          {
            name: "-net-mirror",
            requiresSeparator: true,
            description:
              "Consult the given network mirror (given as a base URL) instead of the origin registry",
            args: {
              name: "url",
            },
          },
          {
            name: "-platform",
            requiresSeparator: true,
            description:
              "Choose a target platform to request package checksums for. Can be used multiple times",
            isRepeatable: true,
            args: {
              name: "os_arch",
              suggestions: [
                "linux_amd64",
                "linux_arm64",
                "darwin_amd64",
                "darwin_arm64",
                "windows_amd64",
              ],
            },
          },
          {
            name: "-enable-plugin-cache",
            description:
              "Enable the usage of the globally configured plugin cache",
          },
          testDirectoryOption,
          ...varOptions,
          helpOption,
        ],
      },
      {
        name: "mirror",
        description: "Save local copies of all required provider plugins",
        args: {
          name: "target-dir",
          template: "folders",
        },
        options: [
          {
            name: "-platform",
            requiresSeparator: true,
            description:
              "Choose which target platform to build a mirror for. Can be used multiple times",
            isRepeatable: true,
            args: {
              name: "os_arch",
              suggestions: [
                "linux_amd64",
                "linux_arm64",
                "darwin_amd64",
                "darwin_arm64",
                "windows_amd64",
              ],
            },
          },
          {
            name: "-lock-file",
            requiresSeparator: true,
            description:
              "Use the provider lock file when fetching providers. Defaults to true",
            args: booleanArg("true or false"),
          },
          ...varOptions,
          helpOption,
        ],
      },
      {
        name: "schema",
        description: "Show schemas for the providers used in the configuration",
        options: [
          jsonOption(
            "Print a JSON representation of the schemas for all providers used in the current configuration",
          ),
          ...varOptions,
          helpOption,
        ],
      },
    ],
  },
  {
    name: "query",
    description: "Search and list remote infrastructure with Terraform",
    options: [
      ...varOptions,
      generateConfigOutOption(
        "Generate import and resource blocks for any found results, written to a new file at PATH",
      ),
      jsonOption(
        "If specified, machine readable output will be printed in JSON format",
      ),
      noColorOption,
      helpOption,
    ],
  },
  {
    name: "refresh",
    description: "Update the state to match remote systems",
    options: [
      compactWarningsOption,
      inputOption,
      ...lockOptions,
      noColorOption,
      parallelismOption,
      targetOption,
      ...varOptions,
      ...legacyStateOptions,
      helpOption,
    ],
  },
  {
    name: "show",
    description: "Show the current state or a saved plan",
    args: {
      name: "path",
      isOptional: true,
      template: "filepaths",
    },
    options: [
      noColorOption,
      jsonOption(
        "If specified, output the Terraform plan or state in a machine-readable form",
      ),
      helpOption,
    ],
  },
  {
    name: "stacks",
    description: "Manage HCP Terraform stack operations",
    args: {
      name: "args",
      isOptional: true,
      isVariadic: true,
    },
  },
  {
    name: "state",
    description: "Advanced state management",
    options: [helpOption],
    subcommands: [
      {
        name: "identities",
        description: "List the identities of resources in the state",
        args: {
          name: "address",
          isOptional: true,
          isVariadic: true,
          generators: addressList,
        },
        options: [
          jsonOption("Output the identities in JSON format (required)"),
          stateOption(
            "Path to a Terraform state file to use to look up Terraform-managed resources",
          ),
          {
            name: "-id",
            requiresSeparator: true,
            description:
              'Filter the results to include only instances whose "id" attribute equals the given string',
            args: {
              name: "ID",
            },
          },
          helpOption,
        ],
      },
      {
        name: "list",
        description: "List resources in the state",
        args: {
          name: "address",
          isOptional: true,
          isVariadic: true,
          generators: addressList,
        },
        options: [
          stateOption(
            "Path to a Terraform state file to use to look up Terraform-managed resources",
          ),
          {
            name: "-id",
            requiresSeparator: true,
            description:
              'Filter the results to include only instances whose "id" attribute equals the given string',
            args: {
              name: "ID",
            },
          },
          helpOption,
        ],
      },
      {
        name: "mv",
        description: "Move an item in the state",
        args: [
          {
            name: "SOURCE",
            generators: addressList,
          },
          {
            name: "DESTINATION",
          },
        ],
        options: [
          {
            name: "-dry-run",
            description:
              "Print out what would've been moved but don't actually move anything",
          },
          ...lockOptions,
          ignoreRemoteVersionOption,
          ...varOptions,
          ...legacyStateOptions,
          helpOption,
        ],
      },
      {
        name: "pull",
        description: "Pull current state and output to stdout",
        options: [...varOptions, helpOption],
      },
      {
        name: "push",
        description: "Update remote state from a local state file",
        args: {
          name: "PATH",
          template: "filepaths",
        },
        options: [
          {
            name: "-force",
            description:
              "Write the state even if lineages don't match or the remote serial is higher",
          },
          ...lockOptions,
          ...varOptions,
          helpOption,
        ],
      },
      {
        name: "replace-provider",
        description: "Replace provider in the state",
        args: [
          {
            name: "FROM_PROVIDER_FQN",
          },
          {
            name: "TO_PROVIDER_FQN",
          },
        ],
        options: [
          {
            name: "-auto-approve",
            description: "Skip interactive approval",
          },
          ...lockOptions,
          ignoreRemoteVersionOption,
          ...varOptions,
          ...legacyStateOptions,
          helpOption,
        ],
      },
      {
        name: "rm",
        description: "Remove instances from the state",
        args: {
          name: "ADDRESS",
          isVariadic: true,
          generators: addressList,
        },
        options: [
          {
            name: "-dry-run",
            description:
              "Print out what would've been removed but don't actually remove anything",
          },
          {
            name: "-backup",
            requiresSeparator: true,
            description: "Path where Terraform should write the backup state",
            args: {
              name: "PATH",
              template: "filepaths",
            },
          },
          ...lockOptions,
          stateOption(
            "Path to the state file to update. Legacy option for the local backend only",
          ),
          ignoreRemoteVersionOption,
          ...varOptions,
          helpOption,
        ],
      },
      {
        name: "show",
        description: "Show a resource in the state",
        args: {
          name: "ADDRESS",
          generators: addressList,
        },
        options: [
          stateOption(
            "Path to a Terraform state file to use to look up Terraform-managed resources",
          ),
          jsonOption(
            "If specified, output the resource state in a machine-readable form",
          ),
          helpOption,
        ],
      },
    ],
  },
  {
    name: "taint",
    description: "Mark a resource instance as not fully functional",
    options: [
      {
        name: "-allow-missing",
        description:
          "If specified, the command will succeed (exit code 0) even if the resource is missing",
      },
      ...lockOptions,
      ignoreRemoteVersionOption,
      ...varOptions,
      ...legacyStateOptions,
      helpOption,
    ],
    args: {
      generators: addressList,
      name: "address",
    },
  },
  {
    name: "test",
    description: "Execute integration tests for Terraform modules",
    options: [
      {
        name: "-cloud-run",
        requiresSeparator: true,
        description:
          "Execute this test run remotely using HCP Terraform or Terraform Enterprise, given the source of a module in a private registry",
        args: {
          name: "source",
        },
      },
      {
        name: "-filter",
        requiresSeparator: true,
        description:
          "Only execute the given test files. Can be used multiple times",
        isRepeatable: true,
        args: {
          name: "testfile",
          template: "filepaths",
        },
      },
      jsonOption(
        "If specified, machine readable output will be printed in JSON format",
      ),
      {
        name: "-junit-xml",
        requiresSeparator: true,
        description: "Save a test report in JUnit XML format to the given file",
        args: {
          name: "path",
          template: "filepaths",
        },
      },
      noColorOption,
      parallelismOption,
      testDirectoryOption,
      ...varOptions,
      {
        name: "-verbose",
        description:
          "Print the plan or state for each test run block as it executes",
      },
      helpOption,
    ],
  },
  {
    name: "untaint",
    description: "Remove the 'tainted' state from a resource instance",
    args: {
      generators: addressList,
      name: "name",
    },
    options: [
      {
        name: "-allow-missing",
        description:
          "If specified, the command will succeed (exit code 0) even if the resource is missing",
      },
      ...lockOptions,
      ignoreRemoteVersionOption,
      ...legacyStateOptions,
      helpOption,
    ],
  },
  {
    name: "version",
    description: "Show the current Terraform version",
    options: [
      jsonOption("Output the version information as a JSON object"),
      helpOption,
    ],
  },
  {
    name: "workspace",
    description: "Workspace management",
    options: [helpOption],
    subcommands: [
      {
        name: "new",
        insertValue: "new '{cursor}'",
        description: "Create a new workspace",
        args: {
          name: "workspace name",
        },
        options: [
          ...lockOptions,
          stateOption("Copy an existing state file into the new workspace"),
          helpOption,
        ],
      },
      {
        name: "show",
        description: "Show the name of the current workspace",
        options: [helpOption],
      },
      {
        name: "list",
        description: "List workspaces",
        options: [
          jsonOption(
            "If specified, machine readable output will be printed in JSON format",
          ),
          helpOption,
        ],
      },
      {
        name: "delete",
        description: "Delete a workspace",
        args: {
          generators: workspaceList,
          name: "workspace name",
        },
        options: [
          {
            name: "-force",
            description: "Remove a workspace even if it is managing resources",
          },
          ...lockOptions,
          helpOption,
        ],
      },
      {
        name: "select",
        description: "Select a workspace",
        args: {
          name: "workspace name",
          generators: workspaceList,
        },
        options: [
          {
            name: "-or-create",
            requiresSeparator: true,
            description:
              "Create the Terraform workspace if it doesn't exist. Defaults to false",
            args: booleanArg("true or false"),
          },
          helpOption,
        ],
      },
    ],
  },
];

const extraCommands: Fig.Subcommand[] = [
  {
    name: "-install-autocomplete",
    description: "Install bash/zsh tab completion",
  },
  {
    name: "-uninstall-autocomplete",
    description: "Uninstall bash/zsh tab completion",
  },
];

const completionSpec: Fig.Spec = {
  name: "terraform",
  description: "Terraform CLI",
  options: globalOptions,
  parserDirectives: {
    flagsArePosixNoncompliant: true,
  },
  subcommands: [...mainCommands, ...otherCommands, ...extraCommands],
};

export default completionSpec;
