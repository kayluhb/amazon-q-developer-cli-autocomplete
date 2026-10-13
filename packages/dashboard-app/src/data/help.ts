import { CLI_BINARY_NAME, USER_MANUAL_URL } from "@/lib/constants";

const supportSteps = {
  steps: [`Run \`${CLI_BINARY_NAME} doctor\` to automatically debug`],
  links: [
    // {
    //   text: 'Troubleshooting guide',
    //   url: ''
    // },
    {
      text: "User manual",
      url: USER_MANUAL_URL,
    },
  ],
};

export default supportSteps;
