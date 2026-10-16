import { PRODUCT_NAME } from "@/lib/constants";

const generalPreferences = [
  {
    title: "Startup",
    properties: [
      {
        id: "app.launchOnStartup",
        title: "Launch on Start",
        description: `Start ${PRODUCT_NAME} automatically whenever you restart your computer.`,
        type: "boolean",
        default: true,
        popular: false,
      },
      // {
      //   id: "app.preferredTerminal",
      //   title: "Preferred Terminal",
      //   description:
      //     `Choose your preferred terminal for ${PRODUCT_NAME} to launch commands in.`,
      //   type: "select",
      //   options: [
      //     "VS Code",
      //     "iTerm2",
      //     "Hyper",
      //     "Alacritty",
      //     "Kitty",
      //     "Terminal",
      //   ],
      //   default: "Terminal",
      //   popular: false,
      // },
      // {
      //   id: "app.disableAutolaunch",
      //   title: "Open in new shells",
      //   description: "Automatically launch when opening a new shell session",
      //   type: "boolean",
      //   default: true,
      //   inverted: true,
      //   popular: false,
      // },
      {
        id: "app.disableAutoupdates",
        title: "Automatic updates",
        description: "Automatically update when new versions are released.",
        type: "boolean",
        default: false,
        inverted: true,
        popular: false,
      },
      {
        id: "app.hideMenubarIcon",
        title: "Display Menu Bar icon",
        description: `${PRODUCT_NAME} icon will appear in the Menu Bar while ${PRODUCT_NAME} is running.`,
        type: "boolean",
        default: false,
        inverted: true,
        popular: false,
      },
    ],
  },
];

export default generalPreferences;
