import logger from "loglevel";
import { SpecLocationSource } from "@tab/shared/utils";
import {
  executeCommand,
  SETTINGS,
  updateSettings,
} from "@tab/api-bindings-wrappers";
import {
  getSpecPath,
  importSpecFromLocation,
  loadFigSubcommand,
  loadSubcommandCached,
} from "../src/loadSpec";
import * as loadHelpers from "../src/loadHelpers";
import {
  expect,
  it,
  beforeAll,
  describe,
  beforeEach,
  vi,
  Mock,
  afterEach,
} from "vitest";

const { importSpecFromFile } = loadHelpers;

vi.mock("../src/loadHelpers", () => ({
  importSpecFromFile: vi
    .fn()
    .mockResolvedValue({ default: { name: "loadFromFile" } }),
  getPrivateSpec: vi.fn().mockReturnValue(undefined),
  isDiffVersionedSpec: vi.fn(),
}));

vi.mock("@tab/api-bindings-wrappers", async () => ({
  ...(await vi.importActual("@tab/api-bindings-wrappers")),
  executeCommand: vi.fn(),
}));

// TODO: remove this statement and move fig dir to shared
const FIG_DIR = "~/.fig";

beforeAll(() => {
  updateSettings({});
});

describe("getSpecPath", () => {
  const cwd = "test_cwd";

  it("works", async () => {
    expect(await getSpecPath("git", cwd)).toEqual({
      type: SpecLocationSource.GLOBAL,
      name: "git",
    });
  });

  it("works for specs containing a slash in the name", async () => {
    expect(
      await getSpecPath("@withfig/autocomplete-tools", cwd, false),
    ).toEqual({
      type: SpecLocationSource.GLOBAL,
      name: "@withfig/autocomplete-tools",
    });
  });

  it("works for scripts containing a slash in the name", async () => {
    expect(await getSpecPath("@withfig/autocomplete-tools", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      name: "autocomplete-tools",
      path: `${cwd}/@withfig/`,
      repoLocal: true,
    });
  });

  it("works properly with local commands", async () => {
    expect(await getSpecPath("./test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      name: "test",
      path: `${cwd}/`,
      repoLocal: true,
    });
    expect(await getSpecPath("~/test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: `~/`,
      name: "test",
    });
    expect(await getSpecPath("/test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: `/`,
      name: "test",
    });
    expect(await getSpecPath("/dir/test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: `/dir/`,
      name: "test",
    });
    expect(await getSpecPath("~/dir/test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: `~/dir/`,
      name: "test",
    });
    expect(await getSpecPath("./dir/test", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: `${cwd}/dir/`,
      name: "test",
      repoLocal: true,
    });
  });

  it("does not search parent folders for ? when repo-local specs are off", async () => {
    (executeCommand as Mock).mockClear();
    expect(await getSpecPath("?", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      path: "~/",
      name: "_shortcuts",
    });
    expect(executeCommand).not.toHaveBeenCalled();
  });

  it("searches parent folders for ? when repo-local specs are on", async () => {
    updateSettings({ [SETTINGS.LOAD_REPO_LOCAL_SPECS]: true });
    try {
      expect(await getSpecPath("?", cwd)).toEqual({
        type: SpecLocationSource.LOCAL,
        path: `${cwd}/`,
        name: "_shortcuts",
        repoLocal: true,
      });
      expect(executeCommand).toHaveBeenCalled();
    } finally {
      updateSettings({});
    }
  });

  it("works properly with + commands", async () => {
    expect(await getSpecPath("+", cwd)).toEqual({
      type: SpecLocationSource.LOCAL,
      name: "+",
      path: "~/",
    });
  });
});

describe("loadFigSubcommand", () => {
  window.URL.createObjectURL = vi.fn();

  beforeEach(() => {
    (loadHelpers.isDiffVersionedSpec as Mock).mockResolvedValue(false);
    updateSettings({});
  });

  afterEach(() => {
    (loadHelpers.isDiffVersionedSpec as Mock).mockClear();
  });

  it("works with expected input", async () => {
    const result = await loadFigSubcommand({
      name: "path",
      type: SpecLocationSource.LOCAL,
    });
    expect(loadHelpers.isDiffVersionedSpec).toHaveBeenCalledTimes(1);
    expect(result.name).toBe("loadFromFile");
  });

  it("works in dev mode", async () => {
    const devPath = "~/some-folder/";
    const specLocation: Fig.SpecLocation = {
      name: "git",
      type: SpecLocationSource.LOCAL,
    };

    updateSettings({
      [SETTINGS.DEV_COMPLETIONS_FOLDER]: devPath,
      [SETTINGS.DEV_MODE_NPM]: false,
      [SETTINGS.DEV_MODE]: false,
    });
    await loadFigSubcommand(specLocation);
    expect(importSpecFromFile).toHaveBeenLastCalledWith(
      "git",
      `${FIG_DIR}/autocomplete/build/`,
      logger,
    );

    updateSettings({
      [SETTINGS.DEV_COMPLETIONS_FOLDER]: devPath,
      [SETTINGS.DEV_MODE_NPM]: true,
      [SETTINGS.DEV_MODE]: false,
    });
    await loadFigSubcommand(specLocation);
    expect(importSpecFromFile).toHaveBeenLastCalledWith("git", devPath, logger);

    updateSettings({
      [SETTINGS.DEV_COMPLETIONS_FOLDER]: devPath,
      [SETTINGS.DEV_MODE_NPM]: false,
      [SETTINGS.DEV_MODE]: true,
    });
    await loadFigSubcommand(specLocation);
    expect(importSpecFromFile).toHaveBeenLastCalledWith("git", devPath, logger);

    updateSettings({
      [SETTINGS.DEV_COMPLETIONS_FOLDER]: "~/some-folder/",
      [SETTINGS.DEV_MODE_NPM]: false,
      [SETTINGS.DEV_MODE]: true,
    });
    await loadFigSubcommand(specLocation);
    expect(importSpecFromFile).toHaveBeenLastCalledWith("git", devPath, logger);

    expect(loadHelpers.isDiffVersionedSpec).toHaveBeenCalledTimes(4);
  });
});

describe("importSpecFromLocation", () => {
  const repoLocal = {
    name: "configure",
    type: SpecLocationSource.LOCAL,
    path: "/repo/",
    repoLocal: true,
  } as const;

  beforeEach(() => {
    updateSettings({});
    (importSpecFromFile as Mock).mockClear();
  });

  afterEach(() => {
    updateSettings({});
  });

  it("does not load repo-local specs by default", async () => {
    await expect(importSpecFromLocation(repoLocal)).rejects.toThrow(
      /Repo-local specs are off/,
    );
    expect(importSpecFromFile).not.toHaveBeenCalled();
  });

  it("loads repo-local specs when the setting is on", async () => {
    updateSettings({ [SETTINGS.LOAD_REPO_LOCAL_SPECS]: true });
    await importSpecFromLocation(repoLocal);
    expect(importSpecFromFile).toHaveBeenLastCalledWith(
      "configure",
      "/repo/.fig/autocomplete/build/",
      logger,
    );
  });

  it("loads absolute-path specs when the setting is off", async () => {
    const location = await getSpecPath("/opt/tool/run", "/repo");
    await importSpecFromLocation(location);
    expect(importSpecFromFile).toHaveBeenLastCalledWith(
      "run",
      "/opt/tool/.fig/autocomplete/build/",
      logger,
    );
  });
});

describe("loadSubcommandCached", () => {
  it("does not serve a cached repo-local spec after the setting turns off", async () => {
    const location = {
      name: "cached-repo-spec",
      type: SpecLocationSource.LOCAL,
      path: "/repo/",
      repoLocal: true,
    } as const;
    updateSettings({ [SETTINGS.LOAD_REPO_LOCAL_SPECS]: true });
    try {
      await loadSubcommandCached(location);
      updateSettings({});
      await expect(loadSubcommandCached(location)).rejects.toThrow(
        /Repo-local specs are off/,
      );
    } finally {
      updateSettings({});
    }
  });

  // This is broken right now...
  it.todo("works", async () => {
    const oldLoadSpec = loadFigSubcommand;
    (loadFigSubcommand as Mock) = vi.fn();
    (loadFigSubcommand as Mock).mockResolvedValue({ name: "exampleSpec" });
    const context: Fig.ShellContext = {
      currentWorkingDirectory: "",
      currentProcess: "",
      sshPrefix: "",
      environmentVariables: {},
    };

    await loadSubcommandCached(
      { name: "git", type: SpecLocationSource.LOCAL },
      context,
    );
    await loadSubcommandCached(
      { name: "git", type: SpecLocationSource.LOCAL },
      context,
    );
    expect(loadFigSubcommand).toHaveBeenCalledTimes(1);

    await loadSubcommandCached(
      { name: "hg", type: SpecLocationSource.LOCAL },
      context,
    );
    expect(loadFigSubcommand).toHaveBeenCalledTimes(2);
    (loadFigSubcommand as unknown) = oldLoadSpec;
  });
});
