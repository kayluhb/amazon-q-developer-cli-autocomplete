import app from "@tab/autocomplete-app/fig.json";
import type { PrefSection } from "@/components/preference/list";
import type { Action } from "@/types/preferences";

// The autocomplete actions the dashboard shows, by group. The title,
// description, availability and default keys come from fig.json. The other
// fig.json actions (show/hide variants, selectSuggestionN) stay hidden.
const groups: { title: string; actions: string[] }[] = [
  { title: "General", actions: ["toggleHistoryMode", "toggleFuzzySearch"] },
  {
    title: "Appearance",
    actions: [
      "increaseSize",
      "decreaseSize",
      "toggleAutocomplete",
      "toggleDescription",
    ],
  },
  {
    title: "Insertion",
    actions: [
      "insertSelected",
      "insertCommonPrefix",
      "insertCommonPrefixOrNavigateDown",
      "insertCommonPrefixOrInsertSelected",
      "insertSelectedAndExecute",
      "execute",
    ],
  },
  { title: "Navigation", actions: ["navigateUp", "navigateDown"] },
];

// Fields that the dashboard shows differently from fig.json.
const overrides: Record<string, Partial<Action>> = {
  insertCommonPrefix: { title: "Insert common prefix or shake" },
  increaseSize: { description: undefined },
  decreaseSize: { description: undefined },
};

function action(identifier: string): Action {
  const contributed = app.contributes.actions.find(
    (candidate) => candidate.identifier === identifier,
  );
  if (!contributed) {
    throw new Error(`No autocomplete action "${identifier}" in fig.json`);
  }
  return {
    id: `${app.name}.${identifier}`,
    title: contributed.name,
    description: contributed.description,
    availability: contributed.availability,
    type: "keystrokes",
    default: contributed.defaultBindings,
    ...overrides[identifier],
  };
}

const keybindings: PrefSection[] = groups.map(({ title, actions }) => ({
  title,
  actions: actions.map(action),
}));

export default keybindings;
